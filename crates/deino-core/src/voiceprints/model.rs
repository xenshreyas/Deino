//! Voiceprints as matching reads them and Settings lists them.

use chrono::{DateTime, Utc};
use deino_domain::{MeetingId, PersonId, VoiceprintId};
use serde::Serialize;

use super::VoiceprintError;

/// Values in one voiceprint: WeSpeaker ResNet34-LM's embedding size, which
/// the `voiceprints.embedding` CHECK (1024 bytes) enforces too.
pub const VOICEPRINT_DIM: usize = 256;

/// Exemplars kept per person; anarlog matches over up to five, and the
/// spike's thresholds assume the max over that many.
pub const MAX_VOICEPRINTS_PER_PERSON: usize = 5;

/// A voice embedding fit to store: [`VOICEPRINT_DIM`] finite values, and
/// the model that produced them.
#[derive(Clone, Debug, PartialEq)]
pub struct VoiceEmbedding {
    values: Vec<f32>,
    model: String,
}

impl VoiceEmbedding {
    /// Checks a diarized speaker's centroid before it becomes a voiceprint.
    pub fn new(values: Vec<f32>, model: impl Into<String>) -> Result<Self, VoiceprintError> {
        let model = model.into();
        let finite = values.iter().all(|value| value.is_finite());
        if values.len() != VOICEPRINT_DIM || !finite || model.trim().is_empty() {
            return Err(VoiceprintError::Unusable);
        }
        Ok(Self { values, model })
    }

    /// The embedding's values.
    pub fn values(&self) -> &[f32] {
        &self.values
    }

    /// The model that produced it.
    pub fn model(&self) -> &str {
        &self.model
    }
}

/// How much clean, single-speaker speech backed a voice, in `[0, 1]`.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Quality(f64);

impl Quality {
    /// The quality `value`, clamped into `[0, 1]` (NaN is `0`).
    pub fn new(value: f64) -> Self {
        if value.is_nan() {
            return Self(0.0);
        }
        Self(value.clamp(0.0, 1.0))
    }

    /// The value.
    pub fn get(self) -> f64 {
        self.0
    }
}

/// One confirmed exemplar of a person's voice, to store.
#[derive(Clone, Debug, PartialEq)]
pub struct Voiceprint {
    /// The voiceprint's ID.
    pub id: VoiceprintId,
    /// Whose voice it is.
    pub person_id: PersonId,
    /// The voice.
    pub embedding: VoiceEmbedding,
    /// The meeting where the user confirmed it.
    pub source_meeting_id: Option<MeetingId>,
    /// How well the voice was heard there.
    pub quality: Quality,
    /// When it was enrolled.
    pub created_at: DateTime<Utc>,
}

/// One stored exemplar, as voice matching reads it.
#[derive(Clone, Debug, PartialEq)]
pub struct Exemplar {
    /// Whose voice it is.
    pub person_id: PersonId,
    /// True when that person is the user.
    pub is_me: bool,
    /// The voice, L2-normalised.
    pub embedding: Vec<f32>,
}

/// A person with enrolled voiceprints, for Settings → People → Voice profiles.
#[derive(Clone, Debug, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct VoiceProfile {
    /// The person.
    pub person_id: PersonId,
    /// Their name.
    pub name: String,
    /// True for the user's own voice.
    pub is_me: bool,
    /// Their exemplars, best first.
    pub voiceprints: Vec<VoiceprintSummary>,
}

/// One exemplar as Settings lists it: where it came from, never the vector.
#[derive(Clone, Debug, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct VoiceprintSummary {
    /// The voiceprint's ID.
    pub id: VoiceprintId,
    /// The meeting it was confirmed in, if that meeting still exists.
    pub source_meeting_id: Option<MeetingId>,
    /// That meeting's title.
    pub source_meeting_title: Option<String>,
    /// How well the voice was heard, in `[0, 1]`.
    pub quality: f64,
    /// When it was enrolled.
    pub created_at: DateTime<Utc>,
}

/// One row of the profile listing: a person and one of their exemplars.
#[derive(Clone, Debug, PartialEq)]
pub struct ProfileRow {
    /// The person.
    pub person_id: PersonId,
    /// Their name.
    pub name: String,
    /// True for the user.
    pub is_me: bool,
    /// The exemplar.
    pub voiceprint: VoiceprintSummary,
}

/// Groups listing rows (sorted by person, then best exemplar first) into
/// one profile per person.
pub fn group_profiles(rows: Vec<ProfileRow>) -> Vec<VoiceProfile> {
    let mut profiles: Vec<VoiceProfile> = Vec::new();
    for row in rows {
        match profiles.last_mut() {
            Some(profile) if profile.person_id == row.person_id => {
                profile.voiceprints.push(row.voiceprint);
            }
            _ => profiles.push(VoiceProfile {
                person_id: row.person_id,
                name: row.name,
                is_me: row.is_me,
                voiceprints: vec![row.voiceprint],
            }),
        }
    }
    profiles
}

#[cfg(test)]
mod tests {
    use deino_domain::{Clock, FakeClock};
    use deino_testkit::at;

    use super::*;

    #[test]
    fn only_full_finite_vectors_from_a_named_model_are_usable() {
        assert!(VoiceEmbedding::new(vec![0.1; VOICEPRINT_DIM], "wespeaker").is_ok());
        assert!(VoiceEmbedding::new(vec![0.1; 3], "wespeaker").is_err());
        let mut nan = vec![0.1; VOICEPRINT_DIM];
        nan[7] = f32::NAN;
        assert!(VoiceEmbedding::new(nan, "wespeaker").is_err());
        assert!(VoiceEmbedding::new(vec![0.1; VOICEPRINT_DIM], " ").is_err());
    }

    #[test]
    fn quality_is_clamped_into_the_unit_interval() {
        assert!((Quality::new(1.7).get() - 1.0).abs() < f64::EPSILON);
        assert!(Quality::new(-0.2).get().abs() < f64::EPSILON);
        assert!(Quality::new(f64::NAN).get().abs() < f64::EPSILON);
    }

    #[test]
    fn rows_group_into_one_profile_per_person_in_order() {
        let clock = FakeClock::new(at("2026-10-11T09:00:00Z"));
        let row = |person: PersonId, name: &str| ProfileRow {
            person_id: person,
            name: name.into(),
            is_me: false,
            voiceprint: VoiceprintSummary {
                id: VoiceprintId::generate(&clock),
                source_meeting_id: None,
                source_meeting_title: None,
                quality: 0.5,
                created_at: clock.now(),
            },
        };
        let (alex, maya) = (PersonId::generate(&clock), PersonId::generate(&clock));
        let rows = vec![row(alex, "Alex"), row(alex, "Alex"), row(maya, "Maya")];

        let profiles = group_profiles(rows);

        assert_eq!(profiles.len(), 2);
        assert_eq!(profiles[0].voiceprints.len(), 2);
        assert_eq!(profiles[1].name, "Maya");
    }
}
