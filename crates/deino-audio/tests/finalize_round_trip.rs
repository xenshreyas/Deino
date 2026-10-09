//! End-to-end on macOS: record through the real `AVAssetWriter` CMAF encoder,
//! stop, finalize, and read the three files back with `AVAudioFile`.
//!
//! Audio is pushed faster than real time, stamped by a fake host clock, so
//! the test takes about a second. Acceptance (FND-05): mic and system are
//! aligned within 50 ms after finalize, in their own files and in the mix.
#![cfg(target_os = "macos")]
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "an integration-test crate: helpers fail the test by panicking, like #[test] bodies"
)]

#[path = "support/decode.rs"]
mod decode;

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use chrono::{TimeZone, Utc};
use decode::{decode_with_av_audio_file, peak_near, rms};
use deino_audio::{
    Bytes, CmafEncoderFactory, FakeDiskSpace, FakeHostClock, Frames, HostClock, HostTime, Recorder,
    RecorderDeps, RecordingInputs, RecordingStatus, SystemHostClock, TestSignal, Track,
    finalize_recording,
};
use deino_domain::FakeClock;
use deino_testkit::Fixtures;
use deino_testkit::fixture::{Channel, MeetingKind, PcmTrack};

/// 10 ms blocks, like a capture callback.
const BLOCK: Frames = Frames(480);
/// The writers are awaited every half second of pushed audio.
const CHECKPOINT: u64 = 24_000;
/// Alignment acceptance: 50 ms.
const TOLERANCE: usize = 2_400;
/// Where the clicks are searched: ±200 ms around their expected frame.
const SEARCH_RADIUS: usize = 9_600;
/// How long the test waits for the writers to catch up before failing.
const CATCH_UP: Duration = Duration::from_secs(30);

/// What to play into each track, and when the system track starts.
struct Script {
    microphone: TestSignal,
    system: TestSignal,
    system_starts: Frames,
    length: Frames,
}

/// Records `script` into `dir` through the real encoder and stops.
fn record(dir: &Path, script: &Script) {
    let clock = Arc::new(FakeHostClock::new(HostTime(1_000_000_000)));
    let deps = RecorderDeps {
        encoders: Arc::new(CmafEncoderFactory),
        host_clock: clock.clone(),
        wall_clock: Arc::new(FakeClock::new(
            Utc.with_ymd_and_hms(2026, 10, 9, 9, 0, 0).unwrap(),
        )),
        disk: Arc::new(FakeDiskSpace::new(Bytes::gib(100))),
    };
    let (recorder, mut inputs) = Recorder::start(dir, &deps).unwrap();
    let mut at = Frames::ZERO;
    while at < script.length {
        push(&mut inputs, script, at, clock.now());
        clock.advance(BLOCK.as_duration());
        at += BLOCK;
        if at.0.is_multiple_of(CHECKPOINT) {
            wait_for_writers(&recorder, at);
        }
    }
    let summary = recorder.stop().unwrap();
    assert_eq!(summary.manifest.status, RecordingStatus::Closed);
}

/// Pushes the block at timeline frame `at`, captured `now`, into each track
/// that is playing.
fn push(inputs: &mut RecordingInputs, script: &Script, at: Frames, now: HostTime) {
    let mut block = vec![0.0; BLOCK.as_len()];
    script.microphone.fill(at, &mut block);
    assert!(inputs.microphone.push(&block, now));
    if at >= script.system_starts {
        script.system.fill(at, &mut block);
        assert!(inputs.system.push(&block, now));
    }
}

/// Waits until both encoders have taken everything up to `at`, so the
/// 2 s rings never overflow while the test pushes faster than real time.
fn wait_for_writers(recorder: &Recorder, at: Frames) {
    let wall = SystemHostClock::new();
    let deadline = wall.now().plus(CATCH_UP);
    while Track::ALL
        .iter()
        .any(|track| recorder.appended(*track) < at)
    {
        assert!(wall.now() < deadline, "the writers fell behind");
        std::thread::yield_now();
    }
}

/// Six seconds of 1 s clicks; the system track joins 370 ms late.
fn click_script() -> Script {
    let clicks = |amplitude| TestSignal::Clicks {
        every: Duration::from_secs(1),
        amplitude,
    };
    Script {
        microphone: clicks(0.8),
        system: clicks(0.6),
        system_starts: Frames(17_760),
        length: Frames(6 * 48_000),
    }
}

/// The largest distance, in frames, between where each of the five clicks
/// after t = 0 was played and where it is in any of the files.
fn worst_click_offset(files: [&[f32]; 3]) -> usize {
    (1..6)
        .map(|second| second * 48_000)
        .flat_map(|played| {
            files.map(|file| peak_near(file, played, SEARCH_RADIUS).abs_diff(played))
        })
        .max()
        .unwrap()
}

#[test]
fn microphone_system_and_mix_stay_aligned_within_50_ms() {
    let dir = tempfile::tempdir().unwrap();
    record(dir.path(), &click_script());

    let finalized = finalize_recording(dir.path()).unwrap();

    let mic = decode_with_av_audio_file(&finalized.microphone);
    let system = decode_with_av_audio_file(&finalized.system);
    let mixed = decode_with_av_audio_file(&finalized.mixed);
    for file in [&mic, &system, &mixed] {
        assert!(
            file.len().abs_diff(288_000) <= 1_024,
            "{} frames",
            file.len()
        );
    }
    let worst = worst_click_offset([&mic, &system, &mixed]);
    println!("worst click offset across mic, system and mix: {worst} frames");
    assert!(worst <= TOLERANCE, "{worst} frames apart");
    assert!(
        rms(&system[..17_000]) < 1e-3,
        "the system track starts with epoch silence"
    );
    assert!(finalized.frames.0.abs_diff(288_000) <= 1_024);
}

/// The standup fixture's track on `channel`, as a 48 kHz signal.
fn fixture_signal(fixtures: &Fixtures, channel: Channel) -> TestSignal {
    let meeting = fixtures.meeting(MeetingKind::Standup).unwrap();
    let audio = meeting.audio.as_ref().unwrap();
    let track = PcmTrack::decode(&fixtures.audio_path(audio, channel).unwrap()).unwrap();
    TestSignal::from_pcm(&track.samples, track.sample_rate)
}

#[test]
fn fixture_audio_survives_the_round_trip() {
    let fixtures = Fixtures::load().unwrap();
    let script = Script {
        microphone: fixture_signal(&fixtures, Channel::Mic),
        system: fixture_signal(&fixtures, Channel::System),
        system_starts: Frames::ZERO,
        length: Frames(4 * 48_000),
    };
    let dir = tempfile::tempdir().unwrap();
    record(dir.path(), &script);

    let finalized = finalize_recording(dir.path()).unwrap();

    for (path, signal) in [
        (&finalized.microphone, &script.microphone),
        (&finalized.system, &script.system),
    ] {
        let decoded = decode_with_av_audio_file(path);
        let mut played = vec![0.0; decoded.len()];
        signal.fill(Frames::ZERO, &mut played);
        let (heard, played) = (rms(&decoded), rms(&played));
        assert!(
            (heard - played).abs() <= 0.2 * played + 1e-4,
            "{heard} vs {played}"
        );
    }
}
