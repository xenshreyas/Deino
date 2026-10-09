//! `DiarizeService`: the `diarize` pipeline stage's rules (order 30).
//!
//! 1. Plan the channels ([`plan_channels`]): which track to diarize, with
//!    how many speakers, and whether the microphone is the user.
//! 2. Diarize that track's finalized file through the [`TrackDiarizer`]
//!    port and store its voices as "Speaker 1..N" with their centroids
//!    ([`SpeakerRepo::replace_speakers`], one transaction).
//! 3. Attribute every segment of that channel by time overlap, splitting at
//!    speaker changes ([`attribute`]); give the user's channel to "Me".
//! 4. Write the attributions ([`SegmentWriter::reattribute`], one
//!    transaction).
//!
//! Running it again replaces the previous result, so the stage is
//! idempotent. It replaces every speaker but the user's, so names assigned
//! to the old speakers are reset (CAP-04's `identify_speakers` runs next).

use std::sync::Arc;

use deino_domain::{Channel, Clock, MeetingId, PaletteColor, SegmentId, SpeakerId};

use super::assign::{Attributed, attribute};
use super::plan::{PlannedTrack, plan_channels};
use super::{DiarizationError, DiarizationOutcome, TrackDiarization, TrackDiarizer, TrackRequest};
use crate::meetings::{MeetingAudio, MeetingLifecycleRepo};
use crate::people::AttendeeRepo;
use crate::segments::{
    NewSpeaker, Page, Reattribution, Segment, SegmentPiece, SegmentReader, SegmentWriter, Speaker,
    SpeakerRepo, SpeakerVoice,
};

/// The label of the user's own speaker.
pub const ME_LABEL: &str = "Me";

/// The ports the stage reads and writes through.
#[derive(Clone)]
pub struct DiarizeDeps {
    /// Meeting audio files.
    pub meetings: Arc<dyn MeetingLifecycleRepo>,
    /// Who was invited.
    pub attendees: Arc<dyn AttendeeRepo>,
    /// The transcript.
    pub segments: Arc<dyn SegmentReader>,
    /// Segment attributions.
    pub writer: Arc<dyn SegmentWriter>,
    /// The meeting's speakers.
    pub speakers: Arc<dyn SpeakerRepo>,
    /// The engine.
    pub diarizer: Arc<dyn TrackDiarizer>,
    /// IDs for new speakers and split segments.
    pub clock: Arc<dyn Clock>,
}

/// Diarizes a finished meeting and attributes its transcript.
pub struct DiarizeService {
    deps: DiarizeDeps,
}

impl DiarizeService {
    /// Creates the service over its ports.
    pub fn new(deps: DiarizeDeps) -> Self {
        Self { deps }
    }

    /// Runs the stage for `meeting`.
    pub async fn run(&self, meeting: &MeetingId) -> Result<DiarizationOutcome, DiarizationError> {
        let state = self.deps.meetings.state(meeting).await?;
        let audio = state
            .ok_or_else(|| DiarizationError::NotFound(meeting.clone()))?
            .audio;
        let segments = self.deps.segments.list(meeting, Page::ALL).await?;
        let attendees = self.deps.attendees.attendees(meeting).await?;
        let plan = plan_channels(&segments, &attendees);
        let mut outcome = DiarizationOutcome::default();
        let mut changes = Vec::new();
        if let Some(track) = plan.diarize {
            let voices = self.diarize(meeting, &audio, track).await?;
            outcome.channel = Some(track.channel);
            outcome.speakers = voices.ids.len();
            changes.extend(self.attribute_channel(&segments, track.channel, &voices));
        }
        if let Some(channel) = plan.me {
            let me = self.me(meeting).await?;
            changes.extend(on(&segments, channel).map(|segment| whole_to(segment, me)));
        }
        outcome.segments = changes.len();
        outcome.splits = changes
            .iter()
            .filter(|change| !change.tail.is_empty())
            .count();
        self.deps.writer.reattribute(&changes).await?;
        log::info!(
            target: "deino_core::diarization",
            "diarized meeting {meeting}: {} speakers on {:?}, {} segments, {} split",
            outcome.speakers, outcome.channel, outcome.segments, outcome.splits
        );
        Ok(outcome)
    }

    /// Runs the engine on the planned track and stores its voices.
    async fn diarize(
        &self,
        meeting: &MeetingId,
        audio: &MeetingAudio,
        track: PlannedTrack,
    ) -> Result<Voices, DiarizationError> {
        let path = track_file(audio, track.channel)
            .ok_or(DiarizationError::AudioMissing(track.channel))?;
        let request = TrackRequest {
            audio: path.into(),
            speakers: track.speakers,
        };
        let diarization = self.deps.diarizer.diarize(request).await?;
        let speakers = self.new_speakers(meeting, &diarization);
        self.deps
            .speakers
            .replace_speakers(meeting, &speakers)
            .await?;
        let ids = speakers.iter().map(|new| new.speaker.id).collect();
        Ok(Voices { ids, diarization })
    }

    /// "Speaker 1..N" in order of first appearance, with their voices.
    fn new_speakers(&self, meeting: &MeetingId, diarization: &TrackDiarization) -> Vec<NewSpeaker> {
        let model = self.deps.diarizer.embedding_model();
        diarization
            .voices
            .iter()
            .enumerate()
            .map(|(index, voice)| NewSpeaker {
                speaker: Speaker {
                    id: SpeakerId::generate(self.deps.clock.as_ref()),
                    meeting_id: meeting.clone(),
                    label: format!("Speaker {}", index + 1),
                    person_id: None,
                    is_me: false,
                    color: PaletteColor::cycle(index + 1),
                },
                voice: Some(SpeakerVoice {
                    centroid: voice.centroid.clone(),
                    embedding_model: model.to_owned(),
                    speech: voice.speech,
                }),
            })
            .collect()
    }

    /// The attributions of every segment on `channel`.
    fn attribute_channel(
        &self,
        segments: &[Segment],
        channel: Channel,
        voices: &Voices,
    ) -> Vec<Reattribution> {
        on(segments, channel)
            .map(|segment| {
                let mut pieces = attribute(segment, &voices.diarization.turns)
                    .into_iter()
                    .map(|piece| voices.piece(piece));
                let head = pieces.next().unwrap_or_else(|| whole_piece(segment, None));
                let tail = pieces
                    .map(|piece| (SegmentId::generate(self.deps.clock.as_ref()), piece))
                    .collect();
                Reattribution {
                    id: segment.id,
                    head,
                    tail,
                }
            })
            .collect()
    }

    /// The user's speaker, created on first use.
    async fn me(&self, meeting: &MeetingId) -> Result<SpeakerId, DiarizationError> {
        let speakers = self.deps.speakers.speakers(meeting).await?;
        if let Some(me) = speakers.iter().find(|speaker| speaker.is_me) {
            return Ok(me.id);
        }
        let me = Speaker {
            id: SpeakerId::generate(self.deps.clock.as_ref()),
            meeting_id: meeting.clone(),
            label: ME_LABEL.to_owned(),
            person_id: None,
            is_me: true,
            color: PaletteColor::cycle(0),
        };
        self.deps.speakers.add_speaker(&me).await?;
        Ok(me.id)
    }
}

/// The stored speakers of a diarized track, by voice index.
struct Voices {
    ids: Vec<SpeakerId>,
    diarization: TrackDiarization,
}

impl Voices {
    fn piece(&self, attributed: Attributed) -> SegmentPiece {
        SegmentPiece {
            speaker_id: attributed
                .voice
                .and_then(|voice| self.ids.get(voice).copied()),
            start: attributed.start,
            end: attributed.end,
            text: attributed.text,
            words: attributed.words,
        }
    }
}

/// The finalized file holding `channel`.
fn track_file(audio: &MeetingAudio, channel: Channel) -> Option<&str> {
    match channel {
        Channel::Mic => audio.audio_mic_path.as_deref(),
        Channel::System => audio.audio_system_path.as_deref(),
        Channel::Mixed => audio.audio_path.as_deref(),
    }
}

fn on(segments: &[Segment], channel: Channel) -> impl Iterator<Item = &Segment> {
    segments
        .iter()
        .filter(move |segment| segment.channel == channel)
}

fn whole_piece(segment: &Segment, speaker: Option<SpeakerId>) -> SegmentPiece {
    SegmentPiece {
        speaker_id: speaker,
        start: segment.start,
        end: segment.end,
        text: segment.text.clone(),
        words: segment.words.clone(),
    }
}

fn whole_to(segment: &Segment, speaker: SpeakerId) -> Reattribution {
    Reattribution {
        id: segment.id,
        head: whole_piece(segment, Some(speaker)),
        tail: Vec::new(),
    }
}

#[cfg(test)]
#[path = "service_tests.rs"]
mod tests;
