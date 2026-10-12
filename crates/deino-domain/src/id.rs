//! Typed identifiers, one newtype per entity.
//!
//! IDs never travel as bare strings: a function that takes a [`MeetingId`]
//! cannot be handed a person's ID by mistake. Every ID serializes as a plain
//! string (`serde(transparent)`), is stored as TEXT (`sqlx(transparent)`) and
//! is exported to TypeScript as a branded string type by the app's bindings
//! test, so the UI cannot mix them up either.
//!
//! New entities get UUIDv7 IDs declared with `uuid_id!` in this file; their
//! time-ordered prefix keeps B-tree inserts local and sorts by creation time.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use uuid::fmt::Hyphenated;
use uuid::{NoContext, Timestamp, Uuid};

use crate::Clock;

/// A string that is not a valid ID.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("{kind} must be a UUID, got {value:?}")]
pub struct IdError {
    /// The ID type that rejected the value, for example `"ExampleNoteId"`.
    pub kind: &'static str,
    /// The rejected input.
    pub value: String,
}

/// Declares a UUIDv7 ID newtype with its serde, sqlx and specta impls.
macro_rules! uuid_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(
            Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord,
            Serialize, Deserialize, specta::Type, sqlx::Type,
        )]
        #[serde(transparent)]
        #[sqlx(transparent)]
        pub struct $name(Hyphenated);

        impl $name {
            /// Creates a new ID stamped with the clock's current time.
            pub fn generate(clock: &dyn Clock) -> Self {
                Self(new_v7(clock).hyphenated())
            }

            /// The underlying UUID.
            pub fn as_uuid(&self) -> Uuid {
                self.0.into_uuid()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(formatter)
            }
        }

        impl FromStr for $name {
            type Err = IdError;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                Uuid::parse_str(value).map(|uuid| Self(uuid.hyphenated())).map_err(|_| IdError {
                    kind: stringify!($name),
                    value: value.to_owned(),
                })
            }
        }
    };
}

/// A UUIDv7 whose timestamp comes from `clock`; the remaining bits are random.
fn new_v7(clock: &dyn Clock) -> Uuid {
    let now = clock.now();
    // UUIDv7 cannot encode times before 1970; a clock that far off clamps to the epoch.
    let seconds = u64::try_from(now.timestamp()).unwrap_or_default();
    Uuid::new_v7(Timestamp::from_unix(
        NoContext,
        seconds,
        now.timestamp_subsec_nanos(),
    ))
}

#[cfg(debug_assertions)]
uuid_id!(
    /// Identifies a note of the `_example` reference feature (debug builds only).
    ExampleNoteId
);

/// Declares an opaque text ID newtype, for IDs whose format Deino does not own.
macro_rules! text_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(
            Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord,
            Serialize, Deserialize, specta::Type, sqlx::Type,
        )]
        #[serde(transparent)]
        #[sqlx(transparent)]
        pub struct $name(String);

        impl $name {
            /// Wraps an existing ID.
            pub fn new(id: impl Into<String>) -> Self {
                Self(id.into())
            }

            /// The ID as stored in the database.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(&self.0)
            }
        }
    };
}

text_id!(
    /// Identifies a meeting.
    ///
    /// Meetings predate UUIDv7 IDs: existing rows use `meeting-<UUIDv4>`, and
    /// the legacy recording and import paths still create IDs in that format.
    /// FND-07 keeps the ID opaque text so every existing meeting still opens;
    /// new meetings may use any unique text, including `meeting-<UUIDv7>`.
    MeetingId
);

text_id!(
    /// Identifies a recurring calendar series, as the calendar provider names it.
    SeriesId
);

text_id!(
    /// Identifies a note template (NOT-04).
    ///
    /// Built-in templates have readable, stable slugs (`one_on_one`), so a
    /// release can upgrade them in place; the user's own templates get a
    /// UUIDv7 from [`TemplateId::generate`]. Both are opaque text to the
    /// database (`templates.id`).
    TemplateId
);

impl TemplateId {
    /// A new ID for one of the user's own templates, stamped with the clock's time.
    pub fn generate(clock: &dyn Clock) -> Self {
        Self(new_v7(clock).hyphenated().to_string())
    }
}

text_id!(
    /// Identifies a stage of the post-meeting pipeline (FND-08), for example
    /// `enhance_notes`. Stages name themselves in code
    /// (`deino-pipeline/src/registry.rs`); `jobs.stage` stores the name.
    StageId
);

uuid_id!(
    /// Identifies a transcript segment. Segments migrated from the legacy
    /// `transcripts` table keep their UUIDv4.
    SegmentId
);

uuid_id!(
    /// Identifies a speaker within one meeting's transcript.
    SpeakerId
);

uuid_id!(
    /// Identifies a person.
    PersonId
);

uuid_id!(
    /// Identifies a company.
    CompanyId
);

uuid_id!(
    /// Identifies a connected calendar account.
    CalendarAccountId
);

uuid_id!(
    /// Identifies one calendar inside a calendar account ("Work", "Family").
    CalendarId
);

uuid_id!(
    /// Identifies a calendar event synced from a calendar account.
    CalendarEventId
);

uuid_id!(
    /// Identifies a folder.
    FolderId
);

uuid_id!(
    /// Identifies a folder's auto-add rule (ORG-05).
    FolderRuleId
);

uuid_id!(
    /// Identifies a tag.
    TagId
);

uuid_id!(
    /// Identifies an action item.
    ActionItemId
);

uuid_id!(
    /// Identifies a saved view of the Action Items page (AI-05).
    ActionItemViewId
);

uuid_id!(
    /// Identifies a saved view of the Meetings page (ORG-07).
    MeetingViewId
);

uuid_id!(
    /// Identifies one LLM provider call in the `llm_calls` usage log.
    LlmCallId
);

uuid_id!(
    /// Identifies one enrolled exemplar of a person's voice (CAP-04).
    VoiceprintId
);

uuid_id!(
    /// Identifies a connected integration account (INT-00).
    ConnectionId
);

uuid_id!(
    /// Identifies one delivery to an integration, in the delivery log (INT-00).
    DeliveryId
);

uuid_id!(
    /// Identifies a custom vocabulary term or replacement rule (TRN-05).
    VocabTermId
);

uuid_id!(
    /// Identifies an insight the user defined (AI-06): what to extract from
    /// meetings, and when.
    InsightId
);

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};

    use super::*;
    use crate::FakeClock;

    uuid_id!(
        /// An ID type for these tests, independent of any feature.
        TestId
    );

    fn clock() -> FakeClock {
        FakeClock::new(Utc.with_ymd_and_hms(2026, 10, 8, 9, 30, 0).unwrap())
    }

    #[test]
    fn generated_ids_are_v7_and_carry_the_clock_time() {
        let id = TestId::generate(&clock());

        let uuid = id.as_uuid();
        assert_eq!(uuid.get_version_num(), 7);
        let (seconds, _) = uuid.get_timestamp().unwrap().to_unix();
        assert_eq!(seconds, 1_791_451_800);
    }

    #[test]
    fn ids_serialize_as_plain_strings_and_round_trip() {
        let id = TestId::generate(&clock());

        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, format!("\"{id}\""));
        assert_eq!(serde_json::from_str::<TestId>(&json).unwrap(), id);
        assert_eq!(id.to_string().parse::<TestId>().unwrap(), id);
    }

    #[test]
    fn parsing_rejects_non_uuids_with_the_type_name() {
        let error = "meeting-1".parse::<TestId>().unwrap_err();
        assert_eq!(error.kind, "TestId");
    }

    #[test]
    fn generated_template_ids_are_v7_uuids_in_text_form() {
        let id = TemplateId::generate(&clock());

        let uuid = Uuid::parse_str(id.as_str()).unwrap();
        assert_eq!(uuid.get_version_num(), 7);
    }

    #[test]
    fn meeting_ids_keep_the_legacy_text_format() {
        let id = MeetingId::new("meeting-0b5f");
        assert_eq!(serde_json::to_string(&id).unwrap(), r#""meeting-0b5f""#);
        assert_eq!(id.to_string(), "meeting-0b5f");
    }
}
