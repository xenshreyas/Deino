//! The tables whose changes the UI can subscribe to.

use serde::{Deserialize, Serialize};

/// A table in Deino's database, as named in SQL.
///
/// The UI tags each query with the tables it reads, and a `DbChanged` event
/// invalidates the queries whose tables changed. A card that creates a table
/// adds its variant here in the same PR.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, specta::Type,
)]
#[serde(rename_all = "snake_case")]
pub enum Table {
    /// Meeting rows: title, status, timing and links.
    Meetings,
    /// Transcript segments, one row per utterance. Legacy code still
    /// reaches them through the `transcripts` compatibility view (ADR 0012),
    /// whose writes are reported as this table.
    TranscriptSegments,
    /// The speakers of each meeting's transcript.
    Speakers,
    /// The user's own notes for a meeting.
    MeetingNotes,
    /// The AI's enhanced notes for a meeting, one row per version (NOT-02).
    EnhancedNotes,
    /// People the user meets.
    People,
    /// Companies people work at.
    Companies,
    /// Who attended which meeting.
    MeetingAttendees,
    /// Connected calendar accounts.
    CalendarAccounts,
    /// Events synced from calendar accounts.
    CalendarEvents,
    /// The calendars inside each calendar account (ORG-01).
    Calendars,
    /// Folders, nested through `parent_id`.
    Folders,
    /// Which meetings sit in which folders.
    MeetingFolders,
    /// Folders' auto-add rules (ORG-05).
    FolderRules,
    /// Tags.
    Tags,
    /// Which tags are applied to which meetings.
    MeetingTags,
    /// Action items, from meetings or standalone.
    ActionItems,
    /// Saved views (filters) of the Action Items page (AI-05).
    ActionItemViews,
    /// One row per LLM provider call: usage, never content.
    LlmCalls,
    /// Post-meeting pipeline jobs: one row per meeting and stage (FND-08).
    Jobs,
    /// The attempts of each pipeline job.
    JobRuns,
    /// One row per outgoing HTTP request: service, host, size, never content.
    EgressLog,
    /// Note templates, built-in and the user's own (NOT-04).
    Templates,
    /// The ordered sections of each template.
    TemplateSections,
    /// Confirmed exemplars of people's voices (CAP-04).
    Voiceprints,
    /// "Is this Alex?" suggestions for unnamed speakers (CAP-04).
    SpeakerSuggestions,
    /// Connected integration accounts, without their tokens (INT-00).
    IntegrationConnections,
    /// The scopes each integration connection was granted.
    IntegrationConnectionScopes,
    /// The delivery log: what Deino sent to which integration (INT-00).
    IntegrationDeliveries,
    /// Notes of the `_example` reference feature, which lives in an in-memory
    /// database in debug builds only.
    ExampleNotes,
}

impl Table {
    /// Every table, for consumers that must assume everything changed.
    pub const ALL: [Self; 30] = [
        Self::Meetings,
        Self::TranscriptSegments,
        Self::Speakers,
        Self::MeetingNotes,
        Self::EnhancedNotes,
        Self::People,
        Self::Companies,
        Self::MeetingAttendees,
        Self::CalendarAccounts,
        Self::CalendarEvents,
        Self::Calendars,
        Self::Folders,
        Self::MeetingFolders,
        Self::FolderRules,
        Self::Tags,
        Self::MeetingTags,
        Self::ActionItems,
        Self::ActionItemViews,
        Self::LlmCalls,
        Self::Jobs,
        Self::JobRuns,
        Self::EgressLog,
        Self::Templates,
        Self::TemplateSections,
        Self::Voiceprints,
        Self::SpeakerSuggestions,
        Self::IntegrationConnections,
        Self::IntegrationConnectionScopes,
        Self::IntegrationDeliveries,
        Self::ExampleNotes,
    ];

    /// The table with this SQL name, if the UI tracks it.
    ///
    /// Runs inside SQLite's update hook for every changed row, so it neither
    /// allocates nor logs.
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "meetings" => Self::Meetings,
            "transcript_segments" => Self::TranscriptSegments,
            "speakers" => Self::Speakers,
            "meeting_notes" => Self::MeetingNotes,
            "enhanced_notes" => Self::EnhancedNotes,
            "people" => Self::People,
            "companies" => Self::Companies,
            "meeting_attendees" => Self::MeetingAttendees,
            "calendar_accounts" => Self::CalendarAccounts,
            "calendar_events" => Self::CalendarEvents,
            "calendars" => Self::Calendars,
            "folders" => Self::Folders,
            "meeting_folders" => Self::MeetingFolders,
            "folder_rules" => Self::FolderRules,
            "tags" => Self::Tags,
            "meeting_tags" => Self::MeetingTags,
            "action_items" => Self::ActionItems,
            "action_item_views" => Self::ActionItemViews,
            "llm_calls" => Self::LlmCalls,
            "jobs" => Self::Jobs,
            "job_runs" => Self::JobRuns,
            "egress_log" => Self::EgressLog,
            "templates" => Self::Templates,
            "template_sections" => Self::TemplateSections,
            "voiceprints" => Self::Voiceprints,
            "speaker_suggestions" => Self::SpeakerSuggestions,
            "integration_connections" => Self::IntegrationConnections,
            "integration_connection_scopes" => Self::IntegrationConnectionScopes,
            "integration_deliveries" => Self::IntegrationDeliveries,
            "example_notes" => Self::ExampleNotes,
            _ => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_name_matches_the_serialized_name_of_every_table() {
        for table in Table::ALL {
            let json = serde_json::to_value(table).unwrap();
            let name = json.as_str().unwrap();
            assert_eq!(Table::from_name(name), Some(table), "{name}");
        }
    }

    #[test]
    fn internal_tables_and_views_are_not_tracked() {
        assert_eq!(Table::from_name("_sqlx_migrations"), None);
        // A view never reaches the update hook; writes through it report the
        // table underneath (`transcript_segments`).
        assert_eq!(Table::from_name("transcripts"), None);
    }
}
