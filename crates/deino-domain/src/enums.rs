//! The closed sets stored in TEXT columns of the core schema (FND-07).
//!
//! Each enum's names match its column's `CHECK (... IN (...))` list exactly;
//! the column is named in each enum's docs. [`MeetingStatus`](crate::MeetingStatus)
//! has its own module because it is a state machine.

db_enum! {
    /// How a meeting came to exist. Stored in `meetings.source`.
    MeetingSource {
        /// Recorded live in Deino.
        Live = "live",
        /// Imported from an audio or video file.
        Import = "import",
        /// Created through the local API (DEV-03).
        Api = "api",
    }
}

db_enum! {
    /// Which audio a transcript segment was heard on. Stored in
    /// `transcript_segments.channel`.
    Channel {
        /// The user's microphone: "Me".
        Mic = "mic",
        /// System audio from the meeting app: "Them".
        System = "system",
        /// Both, mixed before transcription (legacy capture and imports).
        Mixed = "mixed",
    }
}

db_enum! {
    /// How a person took part in a meeting. Stored in `meeting_attendees.role`.
    AttendeeRole {
        /// Organised the calendar event.
        Organizer = "organizer",
        /// Was invited through the calendar event.
        Invitee = "invitee",
        /// Spoke in the meeting without being invited.
        SpeakerOnly = "speaker_only",
    }
}

db_enum! {
    /// A response to a calendar invitation. Stored in `meeting_attendees.rsvp`
    /// and `calendar_events.my_rsvp`.
    Rsvp {
        /// Going.
        Accepted = "accepted",
        /// Not going.
        Declined = "declined",
        /// Maybe.
        Tentative = "tentative",
        /// Has not answered yet.
        NeedsAction = "needs_action",
    }
}

db_enum! {
    /// Whether an action item is still to do. Stored in `action_items.status`.
    ActionItemStatus {
        /// Still to do.
        Open = "open",
        /// Done; `completed_at` says when.
        Done = "done",
    }
}

db_enum! {
    /// How urgent an action item is. Stored in `action_items.priority`.
    ActionItemPriority {
        /// Whenever there is time.
        Low = "low",
        /// Soon.
        Medium = "medium",
        /// First.
        High = "high",
    }
}

db_enum! {
    /// Who created an action item. Stored in `action_items.created_by`.
    ActionItemOrigin {
        /// Extracted from a transcript by the AI.
        Ai = "ai",
        /// Written by the user.
        User = "user",
    }
}

db_enum! {
    /// Who applied a tag to a meeting. Stored in `meeting_tags.applied_by`.
    TagSource {
        /// The user, by hand.
        User = "user",
        /// The auto-tagger (ORG-06).
        Ai = "ai",
        /// An automation rule (AUT-01).
        Automation = "automation",
    }
}

db_enum! {
    /// What a folder's auto-add rule matches (ORG-05). Stored in
    /// `folder_rules.kind`.
    FolderRuleKind {
        /// Meetings of one recurring calendar series.
        Series = "series",
        /// Meetings carrying one tag.
        Tag = "tag",
        /// Meetings with an attendee from one email domain.
        AttendeeDomain = "attendee_domain",
    }
}

db_enum! {
    /// Where a calendar account's events come from. Stored in
    /// `calendar_accounts.provider`.
    CalendarProvider {
        /// macOS Calendar through `EventKit` (every account the Mac knows).
        Apple = "apple",
        /// Google Calendar API.
        Google = "google",
        /// Microsoft Graph (Outlook, Exchange).
        Microsoft = "microsoft",
    }
}

db_enum! {
    /// The video-call service of a calendar event's conference link. Stored in
    /// `calendar_events.platform`.
    ConferencePlatform {
        /// Zoom.
        Zoom = "zoom",
        /// Google Meet.
        GoogleMeet = "google_meet",
        /// Microsoft Teams.
        Teams = "teams",
        /// Webex.
        Webex = "webex",
        /// Slack huddles.
        Slack = "slack",
        /// Any other recognised link.
        Other = "other",
    }
}

db_enum! {
    /// How the notes render one section of a template. Stored in
    /// `template_sections.format`.
    SectionFormat {
        /// A bulleted list, one idea per bullet.
        Bullets = "bullets",
        /// Prose: one or two short paragraphs.
        Paragraph = "paragraph",
        /// A Markdown table with a header row.
        Table = "table",
        /// A task list (`- [ ]`), one action per line.
        Todo = "todo",
    }
}

db_enum! {
    /// Who a meeting is with, from the user's side: only colleagues, or
    /// someone from outside. Template auto rules match on it (NOT-04);
    /// briefs, follow-up emails and calendar sync use it later.
    Audience {
        /// Everyone else shares one of the user's work email domains.
        Internal = "internal",
        /// At least one attendee is from another domain.
        External = "external",
    }
}

db_enum! {
    /// What a custom vocabulary entry does (TRN-05). Stored in
    /// `vocab_terms.kind`.
    VocabKind {
        /// A word or name spelled as given; its sounds-like variants become it.
        Term = "term",
        /// A plain rule: its variants become its text, nothing else.
        Replacement = "replacement",
    }
}

db_enum! {
    /// Where an insight's run on a meeting stands (AI-06). Stored in
    /// `insight_results.status`.
    InsightStatus {
        /// Asked for; the `insights` pipeline stage has not answered yet.
        Pending = "pending",
        /// Answered: the stored answers are this run's.
        Done = "done",
        /// The last run failed; earlier answers, if any, are kept.
        Failed = "failed",
    }
}

db_enum! {
    /// Where an imported meeting came from (CAP-12). Stored in
    /// `imported_meetings.source`, the de-duplication ledger.
    ImportSource {
        /// Granola's public API.
        Granola = "granola",
        /// Circleback's API.
        Circleback = "circleback",
        /// Fireflies' GraphQL API.
        Fireflies = "fireflies",
        /// Fathom's API.
        Fathom = "fathom",
        /// A folder of JSON or Markdown files on this Mac.
        Folder = "folder",
    }
}

db_enum! {
    /// Why a file could not be imported (CAP-11). Stored in
    /// `watched_files.failure`; shown in Settings → Watched folders.
    ImportFailure {
        /// The file disappeared before it could be read.
        Missing = "missing",
        /// The file is larger than an import may be.
        TooLarge = "too_large",
        /// The file holds no audio track.
        NoAudio = "no_audio",
        /// No decoder reads the file's container or codec.
        Unsupported = "unsupported",
        /// The file's audio is damaged.
        Damaged = "damaged",
        /// Something else failed: writing the audio, or saving the meeting.
        Failed = "failed",
    }
}

db_enum! {
    /// When an insight runs (AI-06). Stored in `insight_defs.apply_mode`.
    InsightApplyMode {
        /// After every meeting its conditions match.
        Automatic = "automatic",
        /// Only when the user runs it.
        Manual = "manual",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db_enum::assert_names_agree;

    #[test]
    fn every_enum_serializes_as_its_schema_name() {
        assert_names_agree(MeetingSource::ALL, MeetingSource::as_str);
        assert_names_agree(Channel::ALL, Channel::as_str);
        assert_names_agree(AttendeeRole::ALL, AttendeeRole::as_str);
        assert_names_agree(Rsvp::ALL, Rsvp::as_str);
        assert_names_agree(ActionItemStatus::ALL, ActionItemStatus::as_str);
        assert_names_agree(ActionItemPriority::ALL, ActionItemPriority::as_str);
        assert_names_agree(ActionItemOrigin::ALL, ActionItemOrigin::as_str);
        assert_names_agree(TagSource::ALL, TagSource::as_str);
        assert_names_agree(SectionFormat::ALL, SectionFormat::as_str);
        assert_names_agree(Audience::ALL, Audience::as_str);
        assert_names_agree(FolderRuleKind::ALL, FolderRuleKind::as_str);
        assert_names_agree(CalendarProvider::ALL, CalendarProvider::as_str);
        assert_names_agree(ConferencePlatform::ALL, ConferencePlatform::as_str);
        assert_names_agree(VocabKind::ALL, VocabKind::as_str);
        assert_names_agree(InsightStatus::ALL, InsightStatus::as_str);
        assert_names_agree(InsightApplyMode::ALL, InsightApplyMode::as_str);
        assert_names_agree(ImportSource::ALL, ImportSource::as_str);
    }

    #[test]
    fn multi_word_values_use_snake_case() {
        assert_eq!(AttendeeRole::SpeakerOnly.as_str(), "speaker_only");
        assert_eq!(Rsvp::NeedsAction.to_string(), "needs_action");
        let parsed: ConferencePlatform = serde_json::from_str(r#""google_meet""#).unwrap();
        assert_eq!(parsed, ConferencePlatform::GoogleMeet);
    }
}
