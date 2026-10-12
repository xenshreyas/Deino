//! One meeting as `identify_speakers` sees it: who may be speaking, and how
//! much each unnamed voice resembles each candidate.
//!
//! - **Voices**: the unnamed speakers other than the user's microphone,
//!   each with a diarized voice of at least [`MIN_IDENTIFIABLE_SPEECH`]
//!   (thinner clusters are phantoms; they can still be named by elimination).
//! - **Candidates**: everyone with exemplars of the voices' model, minus the
//!   people already named in the meeting and, on a call (the microphone is
//!   "Me"), minus the user: their voice is not on the remote track.
//! - **Roster**: the diarized speakers and the attendees expected on their
//!   track, for elimination.

use std::sync::Arc;

use deino_db::DbError;
use deino_domain::{MeetingId, PersonId, Rsvp, SpeakerId};

use super::decide::{Ballot, MIN_IDENTIFIABLE_SPEECH, Roster};
use crate::people::{AttendeeRepo, MeRepo};
use crate::segments::{Speaker, SpeakerRepo, SpeakerVoice};
use crate::voiceprints::ExemplarReader;
use crate::voiceprints::matching::best_score;

/// The ports `identify_speakers` reads through.
#[derive(Clone)]
pub struct IdentifyPorts {
    /// The meeting's speakers and their diarized voices.
    pub speakers: Arc<dyn SpeakerRepo>,
    /// Who was invited.
    pub attendees: Arc<dyn AttendeeRepo>,
    /// Which person is the user.
    pub me: Arc<dyn MeRepo>,
    /// Everyone's voice exemplars.
    pub exemplars: Arc<dyn ExemplarReader>,
}

/// One meeting, ready for the rules.
#[derive(Clone, Debug, PartialEq)]
pub struct Lineup {
    /// The unnamed microphone speaker and the "Me" person, when both exist.
    pub me: Option<(SpeakerId, PersonId)>,
    /// Scores, attendees and the diarized track's roster.
    pub ballot: Ballot<SpeakerId, PersonId>,
}

/// A person's exemplars of the voices' model.
type Candidate = (PersonId, Vec<Vec<f32>>);

impl Lineup {
    /// Reads `meeting`, whose speakers are `speakers` (as the speaker tags
    /// left them). `claimed` speakers hold a name from the meeting app
    /// without a person: they are neither matched nor counted for
    /// elimination.
    pub async fn read(
        ports: &IdentifyPorts,
        meeting: &MeetingId,
        speakers: Vec<Speaker>,
        claimed: &[SpeakerId],
    ) -> Result<Self, DbError> {
        let speakers: Vec<Speaker> = speakers
            .into_iter()
            .filter(|speaker| !claimed.contains(&speaker.id))
            .collect();
        let me = ports.me.me().await?.map(|person| person.id);
        let on_call = speakers.iter().any(|speaker| speaker.is_me);
        let user = me.filter(|_| on_call);
        let attendees = coming(ports, meeting).await?;
        let voices = open_voices(ports, &speakers).await?;
        let mut excluded: Vec<PersonId> = speakers.iter().filter_map(|s| s.person_id).collect();
        excluded.extend(user);
        let candidates = candidates(ports, &voices, &excluded).await?;
        let roster = roster(&speakers, &attendees, user);
        let ballot = Ballot {
            scores: scores(&voices, &candidates),
            attendees,
            roster,
        };
        Ok(Self {
            me: microphone(&speakers, me),
            ballot,
        })
    }
}

/// The meeting's attendees who did not decline.
async fn coming(ports: &IdentifyPorts, meeting: &MeetingId) -> Result<Vec<PersonId>, DbError> {
    let attendees = ports.attendees.attendees(meeting).await?;
    Ok(attendees
        .into_iter()
        .filter(|attendee| attendee.rsvp != Some(Rsvp::Declined))
        .map(|attendee| attendee.person_id)
        .collect())
}

/// Unnamed, non-user speakers with a voice thick enough to identify.
async fn open_voices(
    ports: &IdentifyPorts,
    speakers: &[Speaker],
) -> Result<Vec<(SpeakerId, SpeakerVoice)>, DbError> {
    let mut voices = Vec::new();
    for speaker in speakers
        .iter()
        .filter(|s| !s.is_me && s.person_id.is_none())
    {
        let voice = ports.speakers.voice(&speaker.id).await?;
        if let Some(voice) = voice.filter(|voice| voice.speech >= MIN_IDENTIFIABLE_SPEECH) {
            voices.push((speaker.id, voice));
        }
    }
    Ok(voices)
}

/// Everyone not `excluded` with exemplars of the voices' embedding model.
async fn candidates(
    ports: &IdentifyPorts,
    voices: &[(SpeakerId, SpeakerVoice)],
    excluded: &[PersonId],
) -> Result<Vec<Candidate>, DbError> {
    let Some((_, voice)) = voices.first() else {
        return Ok(Vec::new());
    };
    let mut candidates: Vec<Candidate> = Vec::new();
    for exemplar in ports.exemplars.exemplars(&voice.embedding_model).await? {
        if excluded.contains(&exemplar.person_id) {
            continue;
        }
        match candidates.last_mut() {
            Some((person, prints)) if *person == exemplar.person_id => {
                prints.push(exemplar.embedding);
            }
            _ => candidates.push((exemplar.person_id, vec![exemplar.embedding])),
        }
    }
    Ok(candidates)
}

/// Every comparable `(speaker, person, best exemplar score)`. Voices of
/// another model than the candidates' are skipped.
fn scores(
    voices: &[(SpeakerId, SpeakerVoice)],
    candidates: &[Candidate],
) -> Vec<(SpeakerId, PersonId, f32)> {
    let model = voices
        .first()
        .map(|(_, voice)| voice.embedding_model.as_str());
    voices
        .iter()
        .filter(|(_, voice)| Some(voice.embedding_model.as_str()) == model)
        .flat_map(|(speaker, voice)| {
            candidates.iter().filter_map(move |(person, prints)| {
                let score = best_score(&voice.centroid, prints.iter().map(Vec::as_slice))?;
                Some((*speaker, *person, score))
            })
        })
        .collect()
}

/// The diarized track's speakers and the attendees expected on it: the user
/// is never on a call's remote track.
fn roster(
    speakers: &[Speaker],
    attendees: &[PersonId],
    user: Option<PersonId>,
) -> Roster<SpeakerId, PersonId> {
    let diarized = || speakers.iter().filter(|speaker| !speaker.is_me);
    Roster {
        speakers: diarized().map(|speaker| speaker.id).collect(),
        expected: attendees
            .iter()
            .copied()
            .filter(|person| Some(*person) != user)
            .collect(),
        named: diarized()
            .filter_map(|speaker| speaker.person_id.map(|person| (speaker.id, person)))
            .collect(),
    }
}

/// The user's unnamed microphone speaker, paired with the "Me" person.
fn microphone(speakers: &[Speaker], me: Option<PersonId>) -> Option<(SpeakerId, PersonId)> {
    let speaker = speakers
        .iter()
        .find(|speaker| speaker.is_me && speaker.person_id.is_none())?;
    Some((speaker.id, me?))
}
