//! The cloud branch of [`FinalTranscriptionService::run`] (CAP-14).
//!
//! 1. Leave a transcript the user edited alone, unless they asked for the
//!    cloud on this meeting.
//! 2. Send each track to the service: the microphone as the user, system
//!    audio (or the mixed file) diarized, so a call with several people
//!    keeps them apart. Any failure returns before anything is written.
//!    Progress moves a track's share once its upload comes back.
//! 3. Keep the transcript in place ([`OriginalTranscripts`]), store the
//!    cloud's voices as speakers ([`CloudVoices`]), and replace the
//!    transcript: the microphone keeps the user, other lines take their
//!    voice's speaker, else the one carried over.

use deino_db::DbError;
use deino_domain::{Channel, MeetingId, SpeakerId};
use tokio_util::sync::CancellationToken;

use super::progress::TrackShares;
use super::{
    ChannelSpeakers, CloudError, CloudPass, CloudRequest, CloudSpan, CloudVoices, FinalOutcome,
    FinalPass, FinalSkip, FinalTranscriptionError, FinalTranscriptionService, KeptLine,
    carried_segment,
};
use crate::segments::{NewSegment, Segment};

/// A meeting's tracks and its transcript before the run.
type Before = (Vec<(Channel, String)>, Vec<Segment>);
/// The cloud service for the run, and the run's cancellation and progress.
type Cloud<'a> = (&'a CloudPass, &'a FinalPass);

impl FinalTranscriptionService {
    /// Transcribes `tracks` with `cloud` and replaces `old` with the result.
    pub(super) async fn run_cloud(
        &self,
        meeting: &MeetingId,
        (tracks, old): Before,
        (cloud, pass): Cloud<'_>,
    ) -> Result<FinalOutcome, FinalTranscriptionError> {
        if !cloud.requested && old.iter().any(|segment| segment.edited_at.is_some()) {
            return Ok(FinalOutcome::Skipped(FinalSkip::EditedByUser));
        }
        let shares = TrackShares::new(pass.progress.clone(), tracks.len());
        let heard = hear(cloud, tracks, &pass.cancel, &shares)
            .await
            .map_err(FinalTranscriptionError::Cloud)?;
        if heard.is_empty() {
            return Ok(FinalOutcome::Skipped(FinalSkip::NoSpeech));
        }
        if !old.is_empty() {
            let lines: Vec<KeptLine> = old.iter().map(KeptLine::of).collect();
            let now = self.deps.clock.now();
            self.deps.originals.keep(meeting, &lines, now).await?;
        }
        let rows = self.cloud_rows(meeting, &old, heard).await?;
        self.deps.writer.replace(meeting, &rows).await?;
        Ok(FinalOutcome::Written {
            segments: rows.len(),
            replaced: old.len(),
        })
    }

    /// The new segments: the cloud's voices stored as speakers first, then
    /// each line with its speaker, never one the new speakers replaced.
    async fn cloud_rows(
        &self,
        meeting: &MeetingId,
        old: &[Segment],
        mut heard: Vec<(Channel, CloudSpan)>,
    ) -> Result<Vec<NewSegment>, DbError> {
        heard.sort_by_key(|(_, cloud)| cloud.span.start);
        let kept = self.deps.speakers.speakers(meeting).await?;
        let clock = self.deps.clock.as_ref();
        let voices = CloudVoices::of(meeting, &heard, &kept, clock);
        if !voices.is_empty() {
            let speakers = voices.speakers();
            self.deps
                .speakers
                .replace_speakers(meeting, speakers)
                .await?;
        }
        let now = self.deps.speakers.speakers(meeting).await?;
        let fallback = ChannelSpeakers::of(&now);
        let valid: Vec<SpeakerId> = now.iter().map(|speaker| speaker.id).collect();
        Ok(heard
            .into_iter()
            .map(|(channel, cloud)| {
                let voice = voices.speaker_of(channel, cloud.voice.as_deref());
                let mut row =
                    carried_segment(meeting, (channel, cloud.span), (old, fallback), clock);
                row.speaker_id = voice
                    .or(row.speaker_id)
                    .filter(|speaker| valid.contains(speaker));
                row
            })
            .collect())
    }
}

/// Every utterance of every track, with its channel.
async fn hear(
    cloud: &CloudPass,
    tracks: Vec<(Channel, String)>,
    cancel: &CancellationToken,
    shares: &TrackShares,
) -> Result<Vec<(Channel, CloudSpan)>, CloudError> {
    let mut heard = Vec::new();
    for (index, (channel, path)) in tracks.into_iter().enumerate() {
        let request = CloudRequest {
            audio: path.into(),
            diarize: channel != Channel::Mic,
            cancel: cancel.clone(),
        };
        let spans = cloud.transcriber.transcribe(request).await?;
        shares.done(index);
        heard.extend(spans.into_iter().map(|span| (channel, span)));
    }
    Ok(heard)
}

#[cfg(test)]
#[path = "cloud_run_tests.rs"]
mod tests;
