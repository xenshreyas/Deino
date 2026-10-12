//! [`FinalTranscriptionService`]: the `final_transcription` stage's rules.
//!
//! 1. Find the meeting's finished tracks: microphone and system audio, or
//!    the mixed file of an older recording.
//! 2. Decide: a meeting without a transcript (recorded with live
//!    transcription off) is transcribed; one with a live transcript is
//!    re-transcribed only when the high-accuracy re-pass is on and the user
//!    edited nothing; otherwise the live transcript stays.
//! 3. Transcribe each track through the [`TrackTranscriber`] port.
//! 4. Replace the transcript in one transaction, each utterance keeping the
//!    speaker the old segments gave its time ([`carried_speaker`]).
//!
//! When the run has a cloud service (CAP-14, [`FinalPass::cloud`]), steps 2
//! and 3 are the cloud branch's instead (`cloud_run.rs`): the cloud
//! transcript replaces the live one unless the user edited it, and the
//! replaced one is kept ([`OriginalTranscripts`]).
//!
//! Replacing makes a retry or a re-run safe.

use std::sync::Arc;

use deino_db::DbError;
use deino_domain::{Channel, Clock, ErrorClass, ErrorCode, MeetingId};
use tokio_util::sync::CancellationToken;

#[cfg(doc)]
use super::carried_speaker;
use super::{
    ChannelSpeakers, CloudError, CloudPass, HeardSpan, OriginalTranscripts, TrackError,
    TrackRequest, TrackTranscriber, carried_segment,
};
use crate::meetings::{MeetingAudio, MeetingLifecycleRepo};
use crate::segments::{NewSegment, Page, Segment, SegmentReader, SegmentWriter, SpeakerRepo};

/// Why the stage had nothing to write.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FinalSkip {
    /// The meeting has no recorded audio.
    NoAudio,
    /// The live transcript stays: the re-pass is off.
    LiveTranscriptKept,
    /// The re-pass would replace the user's edits.
    EditedByUser,
    /// Nothing was said on any track.
    NoSpeech,
}

/// What a run did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FinalOutcome {
    /// The transcript was written: `segments` new ones, replacing `replaced`.
    Written {
        /// Segments written.
        segments: usize,
        /// Live segments they replaced.
        replaced: usize,
    },
    /// Nothing was written, for this reason.
    Skipped(FinalSkip),
}

/// Why the stage failed.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum FinalTranscriptionError {
    /// The meeting does not exist.
    #[error("meeting {0} does not exist")]
    NotFound(MeetingId),
    /// A track of a meeting without a transcript could not be transcribed.
    #[error(transparent)]
    Track(TrackError),
    /// The re-pass could not transcribe a track; the live transcript is
    /// untouched.
    #[error(transparent)]
    Repass(TrackError),
    /// The cloud service could not transcribe a track (CAP-14); nothing
    /// was written, so the transcript in place is untouched.
    #[error(transparent)]
    Cloud(CloudError),
    /// The database failed.
    #[error("could not read or save the transcript")]
    Storage(#[from] DbError),
}

impl ErrorClass for FinalTranscriptionError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::NotFound(_) => ErrorCode::NotFound,
            Self::Track(error) | Self::Repass(error) => error.code(),
            Self::Cloud(error) => error.code(),
            Self::Storage(_) => ErrorCode::Internal,
        }
    }
}

/// The ports the stage reads and writes through.
#[derive(Clone)]
pub struct FinalTranscriptionDeps {
    /// The meeting's audio files.
    pub meetings: Arc<dyn MeetingLifecycleRepo>,
    /// The live transcript.
    pub segments: Arc<dyn SegmentReader>,
    /// Where the new transcript goes.
    pub writer: Arc<dyn SegmentWriter>,
    /// The meeting's speakers.
    pub speakers: Arc<dyn SpeakerRepo>,
    /// The engine.
    pub transcriber: Arc<dyn TrackTranscriber>,
    /// Where a transcript the cloud replaces is kept (CAP-14).
    pub originals: Arc<dyn OriginalTranscripts>,
    /// IDs for the new segments, and when a transcript was replaced.
    pub clock: Arc<dyn Clock>,
}

/// One run's request.
#[derive(Clone, Debug)]
pub struct FinalPass {
    /// Settings → Transcription → "High-accuracy re-pass after meeting".
    pub repass: bool,
    /// The cloud service that transcribes this run instead of the Mac
    /// (CAP-14), if the user turned one on or asked for it.
    pub cloud: Option<CloudPass>,
    /// Fires when the stage is cancelled.
    pub cancel: CancellationToken,
}

/// Transcribes finished meetings.
pub struct FinalTranscriptionService {
    pub(super) deps: FinalTranscriptionDeps,
}

impl FinalTranscriptionService {
    /// The service over its ports.
    pub fn new(deps: FinalTranscriptionDeps) -> Self {
        Self { deps }
    }

    /// Runs the stage for `meeting`.
    pub async fn run(
        &self,
        meeting: &MeetingId,
        pass: &FinalPass,
    ) -> Result<FinalOutcome, FinalTranscriptionError> {
        let state = self.deps.meetings.state(meeting).await?;
        let audio = state
            .ok_or_else(|| FinalTranscriptionError::NotFound(meeting.clone()))?
            .audio;
        let tracks = tracks(&audio);
        if tracks.is_empty() {
            return Ok(FinalOutcome::Skipped(FinalSkip::NoAudio));
        }
        let old = self.deps.segments.list(meeting, Page::ALL).await?;
        if let Some(cloud) = &pass.cloud {
            return self
                .run_cloud(meeting, (tracks, old), (cloud, &pass.cancel))
                .await;
        }
        if let Some(skip) = skip(&old, pass.repass) {
            return Ok(FinalOutcome::Skipped(skip));
        }
        let heard = self.hear(tracks, pass).await.map_err(|error| {
            if old.is_empty() {
                FinalTranscriptionError::Track(error)
            } else {
                FinalTranscriptionError::Repass(error)
            }
        })?;
        if heard.is_empty() {
            return Ok(FinalOutcome::Skipped(FinalSkip::NoSpeech));
        }
        let rows = self.rows(meeting, &old, heard).await?;
        self.deps.writer.replace(meeting, &rows).await?;
        Ok(FinalOutcome::Written {
            segments: rows.len(),
            replaced: old.len(),
        })
    }

    /// Every utterance of every track, with its channel.
    async fn hear(
        &self,
        tracks: Vec<(Channel, String)>,
        pass: &FinalPass,
    ) -> Result<Vec<(Channel, HeardSpan)>, TrackError> {
        let mut heard = Vec::new();
        for (channel, path) in tracks {
            let request = TrackRequest {
                audio: path.into(),
                cancel: pass.cancel.clone(),
            };
            let spans = self.deps.transcriber.transcribe(request).await?;
            heard.extend(spans.into_iter().map(|span| (channel, span)));
        }
        Ok(heard)
    }

    /// The new segments, in time order, each with its carried speaker.
    async fn rows(
        &self,
        meeting: &MeetingId,
        old: &[Segment],
        mut heard: Vec<(Channel, HeardSpan)>,
    ) -> Result<Vec<NewSegment>, DbError> {
        heard.sort_by_key(|(_, span)| span.start);
        let fallback = ChannelSpeakers::of(&self.deps.speakers.speakers(meeting).await?);
        let clock = self.deps.clock.as_ref();
        Ok(heard
            .into_iter()
            .map(|heard| carried_segment(meeting, heard, (old, fallback), clock))
            .collect())
    }
}

/// The tracks to transcribe: microphone and system audio, else the mix.
pub(super) fn tracks(audio: &MeetingAudio) -> Vec<(Channel, String)> {
    let split: Vec<(Channel, String)> = [
        (Channel::Mic, audio.audio_mic_path.clone()),
        (Channel::System, audio.audio_system_path.clone()),
    ]
    .into_iter()
    .filter_map(|(channel, path)| Some((channel, path?)))
    .collect();
    if !split.is_empty() {
        return split;
    }
    audio
        .audio_path
        .clone()
        .map(|path| vec![(Channel::Mixed, path)])
        .unwrap_or_default()
}

/// Why a meeting with transcript `old` is left alone, if it is.
fn skip(old: &[Segment], repass: bool) -> Option<FinalSkip> {
    if old.is_empty() {
        None
    } else if !repass {
        Some(FinalSkip::LiveTranscriptKept)
    } else if old.iter().any(|segment| segment.edited_at.is_some()) {
        Some(FinalSkip::EditedByUser)
    } else {
        None
    }
}

#[cfg(test)]
#[path = "service_tests.rs"]
mod tests;
