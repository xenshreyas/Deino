//! [`FollowAlong`]: the after-meeting work, done while the meeting is
//! recorded, so that when it ends only its last moments are left.
//!
//! Every [`PASS_EVERY`] while a meeting is recording (and the live
//! transcript is off, so the engine is free), a pass takes a snapshot of
//! each track as recorded so far (an APFS clone of the growing CMAF file,
//! cut back to its last whole fragment: what a reader of a finished file
//! sees) and runs the stage's own work over it with the shared
//! [`PassMemory`]: the transcriber, the word aligner on the call's lines
//! and, every [`SPEAKERS_EVERY`] passes, the diarizer's scan. Each answer
//! is remembered by the exact audio it came from, so the pass after the
//! meeting reuses it and gives the same transcript as without this.
//!
//! A pass holds the memory's gate, so the pass after the meeting waits for
//! one in flight instead of racing it. Nothing is written to the meeting.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use deino_core::diarization::{
    AlignLine, SpeakerHint, TrackDiarizer, TrackRequest as SpeakerRequest, WordAligner,
};
use deino_core::final_transcription::{HeardSpan, TrackProgress, TrackRequest, TrackTranscriber};
use deino_domain::{Channel, Clock, MeetingId, SegmentId};
use tokio_util::sync::CancellationToken;

use super::{OfflineTranscriber, PassMemory};
use crate::events::RecordingPhase;
use crate::service::RecordingService;

/// How often a pass runs while recording.
pub const PASS_EVERY: Duration = Duration::from_secs(30);

/// Passes between diarizer scans (each loads the speaker models).
pub const SPEAKERS_EVERY: u32 = 4;

/// What is being recorded now.
pub trait Recorder: Send + Sync {
    /// The meeting recording now (not paused), if any.
    fn recording(&self) -> Option<MeetingId>;
}

impl Recorder for RecordingService {
    fn recording(&self) -> Option<MeetingId> {
        let state = self.state();
        state
            .meeting_id
            .filter(|_| state.phase == RecordingPhase::Recording)
    }
}

/// Whether passes should run now (the live transcript is off).
#[async_trait]
pub trait FollowWanted: Send + Sync {
    /// True when the after-meeting pass will transcribe the recording and
    /// nothing else needs the engine while it is made.
    async fn wanted(&self) -> bool;
}

/// What following uses.
pub struct FollowDeps {
    /// What is recording.
    pub recorder: Arc<dyn Recorder>,
    /// The recordings folder (`<meeting>/audio/<track>.001.m4a.part`).
    pub recordings: PathBuf,
    /// The stage's transcriber over the shared memory.
    pub transcriber: OfflineTranscriber,
    /// The memory the pass after the meeting shares.
    pub memory: PassMemory,
    /// The `diarize` stage's word aligner (with its own memory).
    pub aligner: Arc<dyn WordAligner>,
    /// The `diarize` stage's diarizer (with its own memory).
    pub diarizer: Arc<dyn TrackDiarizer>,
    /// Whether to follow at all.
    pub wanted: Arc<dyn FollowWanted>,
    /// Called when a new recording is followed: forget the last one.
    pub forget: Arc<dyn Fn() + Send + Sync>,
    /// New line IDs (the lines are never stored).
    pub clock: Arc<dyn Clock>,
}

/// Follows each recording while it is made.
pub struct FollowAlong {
    deps: FollowDeps,
}

impl FollowAlong {
    /// Following over `deps`.
    pub fn new(deps: FollowDeps) -> Self {
        Self { deps }
    }

    /// Runs passes until `cancel` fires.
    pub async fn run(self, cancel: CancellationToken) {
        let mut followed: Option<(MeetingId, u32)> = None;
        loop {
            tokio::select! {
                () = cancel.cancelled() => return,
                () = tokio::time::sleep(PASS_EVERY) => {}
            }
            let Some(meeting) = self.recording().await else {
                continue;
            };
            let passes = match &followed {
                Some((id, passes)) if *id == meeting => *passes,
                _ => {
                    (self.deps.forget)();
                    0
                }
            };
            if let Err(error) = self.pass(&meeting, passes % SPEAKERS_EVERY == 0).await {
                log::warn!(target: "pipeline", "following {meeting}: {error}");
            }
            followed = Some((meeting, passes + 1));
        }
    }

    /// The meeting recording now, when passes are wanted.
    async fn recording(&self) -> Option<MeetingId> {
        let meeting = self.deps.recorder.recording()?;
        self.deps.wanted.wanted().await.then_some(meeting)
    }

    /// One pass over what `meeting` has recorded so far.
    async fn pass(&self, meeting: &MeetingId, speakers: bool) -> Result<(), String> {
        let audio = self.deps.recordings.join(meeting.to_string()).join("audio");
        let scratch = self.deps.recordings.join(".following");
        std::fs::create_dir_all(&scratch).map_err(|error| error.to_string())?;
        let system = snapshot(&audio, "system", &scratch)?;
        let mic = snapshot(&audio, "mic", &scratch)?;
        self.pass_over((mic, system), speakers).await
    }

    /// One pass over playable copies of the tracks (`mic`, `system`) as
    /// recorded so far, holding off the stage meanwhile; the diarizer's
    /// scan only when `speakers`.
    pub async fn pass_over(
        &self,
        (mic, system): (Option<PathBuf>, Option<PathBuf>),
        speakers: bool,
    ) -> Result<(), String> {
        let _gate = self.deps.memory.exclusive().await;
        let heard = self.transcribe(mic, system.clone()).await?;
        if let Some(system) = system {
            self.align(&system, heard).await?;
            if speakers {
                self.scan(system).await?;
            }
        }
        Ok(())
    }

    /// Transcribes both tracks into the memory; the call's lines.
    async fn transcribe(
        &self,
        mic: Option<PathBuf>,
        system: Option<PathBuf>,
    ) -> Result<Vec<HeardSpan>, String> {
        let request = |audio: PathBuf, channel, echo_reference| TrackRequest {
            audio,
            channel,
            echo_reference,
            cancel: CancellationToken::new(),
            progress: TrackProgress::default(),
        };
        let transcriber = &self.deps.transcriber;
        let call = system
            .clone()
            .map(|path| transcriber.transcribe(request(path, Channel::System, None)));
        let me = mic.map(|path| transcriber.transcribe(request(path, Channel::Mic, system)));
        let (call, me) = tokio::join!(optional(call), optional(me));
        me.transpose().map_err(|error| error.to_string())?;
        Ok(call
            .transpose()
            .map_err(|error| error.to_string())?
            .unwrap_or_default())
    }

    /// Times the words of the call's lines into the aligner's memory.
    async fn align(&self, system: &Path, heard: Vec<HeardSpan>) -> Result<(), String> {
        let lines: Vec<AlignLine> = heard
            .into_iter()
            .map(|span| AlignLine {
                segment: SegmentId::generate(self.deps.clock.as_ref()),
                start: span.start,
                end: span.end,
                text: span.text,
            })
            .collect();
        self.deps
            .aligner
            .align(system, &lines)
            .await
            .map(drop)
            .map_err(|error| error.to_string())
    }

    /// Scans the call's track into the diarizer's memory.
    async fn scan(&self, system: PathBuf) -> Result<(), String> {
        let request = SpeakerRequest {
            audio: system,
            speakers: SpeakerHint::Unknown,
            known: Vec::new(),
        };
        self.deps
            .diarizer
            .diarize(request)
            .await
            .map(drop)
            .map_err(|error| error.to_string())
    }
}

/// `future`'s output when there is one.
async fn optional<T>(future: Option<impl Future<Output = T>>) -> Option<T> {
    match future {
        Some(future) => Some(future.await),
        None => None,
    }
}

/// A playable copy of `track`'s file being recorded, in `scratch`; none
/// when the track is not being recorded in one file (or not at all).
fn snapshot(audio: &Path, track: &str, scratch: &Path) -> Result<Option<PathBuf>, String> {
    let recording = audio.join(format!("{track}.001.m4a.part"));
    if !recording.exists() || audio.join(format!("{track}.002.m4a.part")).exists() {
        return Ok(None);
    }
    let copy = scratch.join(format!("{track}.m4a"));
    std::fs::copy(&recording, &copy).map_err(|error| error.to_string())?;
    let scanned =
        deino_audio::fmp4::truncate_to_playable(&copy).map_err(|error| error.to_string())?;
    Ok((scanned.has_moov && scanned.fragments > 0).then_some(copy))
}

#[cfg(test)]
#[path = "follow_tests.rs"]
mod tests;
