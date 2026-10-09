//! The macOS finalize steps: flatten each track, then mix.

use std::fs::File;
use std::path::{Path, PathBuf};

use super::export::export_passthrough;
use super::ext_audio::{AacWriter, Container, PcmReader};
use super::mix::mix_blocks;
use super::{FinalizeError, FinalizedAudio, MIXED_FILE, track_file};
use crate::durability::{full_fsync, rename_durably};
use crate::manifest::{
    ManifestStore, RecordingManifest, RecordingStatus, SegmentManifest, SegmentState,
};
use crate::units::{Frames, Track};

/// Frames decoded, mixed or written per step (about 170 ms).
const BLOCK_FRAMES: usize = 8_192;

/// Prefix of the temporary names outputs are written under.
const TEMP_PREFIX: &str = ".finalizing.";

/// See [`super::finalize_recording`].
pub(super) fn finalize(dir: &Path) -> Result<FinalizedAudio, FinalizeError> {
    let store = ManifestStore::new(dir);
    let mut manifest = store.read()?;
    if manifest.status.needs_recovery() {
        return Err(FinalizeError::NotStopped(manifest.status));
    }
    let microphone = flatten(dir, &manifest, Track::Microphone)?;
    let system = flatten(dir, &manifest, Track::System)?;
    let mixed = dir.join(MIXED_FILE);
    let frames = mix(&microphone, &system, &mixed)?;
    manifest.status = RecordingStatus::Finalized;
    store.write(&manifest)?;
    Ok(FinalizedAudio {
        microphone,
        system,
        mixed,
        frames,
    })
}

/// Writes one track's flat `.m4a` and returns its path.
fn flatten(
    dir: &Path,
    manifest: &RecordingManifest,
    track: Track,
) -> Result<PathBuf, FinalizeError> {
    let output = dir.join(track_file(track));
    let temp = temp_path(&output);
    remove_if_present(&temp)?;
    let segments: Vec<SegmentManifest> = manifest
        .track(track)
        .map(|entry| entry.segments.iter().copied().filter(is_playable).collect())
        .unwrap_or_default();
    match segments.as_slice() {
        [only] if only.first_frame == Frames::ZERO => {
            export_passthrough(&dir.join(only.relative_path(track)), &temp)?;
        }
        _ => splice(dir, track, &segments, &temp)?,
    }
    publish(&temp, &output)?;
    Ok(output)
}

/// Whether a segment has audio to keep.
fn is_playable(segment: &SegmentManifest) -> bool {
    segment.state == SegmentState::Closed && segment.durable_frames > Frames::ZERO
}

/// Decodes `segments` in order into one file, with silence for the gaps.
fn splice(
    dir: &Path,
    track: Track,
    segments: &[SegmentManifest],
    output: &Path,
) -> Result<(), FinalizeError> {
    let mut writer = AacWriter::create(output, Container::M4a)?;
    let mut position = Frames::ZERO;
    let mut block = vec![0.0; BLOCK_FRAMES];
    for segment in segments {
        position += write_silence(&mut writer, segment.first_frame - position)?;
        let mut reader = PcmReader::open(&dir.join(segment.relative_path(track)))?;
        loop {
            let read = reader.read(&mut block)?;
            let Some(samples) = block.get(..read).filter(|samples| !samples.is_empty()) else {
                break;
            };
            writer.write(samples)?;
            position += Frames::of_len(read);
        }
    }
    writer.close();
    Ok(())
}

/// Writes `frames` of silence; returns `frames`.
fn write_silence(writer: &mut AacWriter, frames: Frames) -> Result<Frames, FinalizeError> {
    let zeros = vec![0.0; BLOCK_FRAMES];
    let mut left = frames;
    while left > Frames::ZERO {
        let n = left.min(Frames::of_len(BLOCK_FRAMES));
        writer.write(zeros.get(..n.as_len()).unwrap_or_default())?;
        left = left - n;
    }
    Ok(frames)
}

/// Mixes the two flat tracks into `output` (`audio.mp4`); returns its length.
fn mix(microphone: &Path, system: &Path, output: &Path) -> Result<Frames, FinalizeError> {
    let temp = temp_path(output);
    remove_if_present(&temp)?;
    let mut mic_reader = PcmReader::open(microphone)?;
    let mut system_reader = PcmReader::open(system)?;
    let mut writer = AacWriter::create(&temp, Container::Mp4)?;
    let (mut mic_block, mut system_block) = (vec![0.0; BLOCK_FRAMES], vec![0.0; BLOCK_FRAMES]);
    let mut mixed = Vec::with_capacity(BLOCK_FRAMES);
    let mut total = Frames::ZERO;
    loop {
        let mic_read = mic_reader.read(&mut mic_block)?;
        let system_read = system_reader.read(&mut system_block)?;
        if mic_read == 0 && system_read == 0 {
            break;
        }
        let mic = mic_block.get(..mic_read).unwrap_or_default();
        let sys = system_block.get(..system_read).unwrap_or_default();
        mix_blocks(mic, sys, &mut mixed);
        writer.write(&mixed)?;
        total += Frames::of_len(mixed.len());
    }
    writer.close();
    publish(&temp, output)?;
    Ok(total)
}

/// The temporary name an output is written under before it is published.
fn temp_path(output: &Path) -> PathBuf {
    let name = output
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    output.with_file_name(format!("{TEMP_PREFIX}{name}"))
}

/// Syncs `temp` and renames it to `output`, durably.
fn publish(temp: &Path, output: &Path) -> Result<(), FinalizeError> {
    full_fsync(&File::open(temp)?)?;
    rename_durably(temp, output)?;
    Ok(())
}

/// Removes a stale temporary file left by an interrupted finalize.
fn remove_if_present(path: &Path) -> Result<(), FinalizeError> {
    match std::fs::remove_file(path) {
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => Err(error.into()),
        _ => Ok(()),
    }
}
