//! The tables that existed when `Table` became generated (REG-01), in their
//! original order and with their original docs, so the enum, its `Ord` and the
//! TypeScript bindings stayed exactly as they were.
//!
//! **Frozen: never add a table here.** A new table is documented by the `--`
//! comment block directly above its `CREATE TABLE` and is appended after
//! these. Remove an entry only when a migration drops its table.

/// `(SQL name, doc lines)`, in the order the variants are declared.
pub const DOCUMENTED: &[(&str, &[&str])] = &[
    (
        "meetings",
        &["Meeting rows: title, status, timing and links."],
    ),
    (
        "transcript_segments",
        &[
            "Transcript segments, one row per utterance. Legacy code still",
            "reaches them through the `transcripts` compatibility view (ADR 0012),",
            "whose writes are reported as this table.",
        ],
    ),
    ("speakers", &["The speakers of each meeting's transcript."]),
    ("meeting_notes", &["The user's own notes for a meeting."]),
    (
        "enhanced_notes",
        &["The AI's enhanced notes for a meeting, one row per version (NOT-02)."],
    ),
    ("people", &["People the user meets."]),
    ("companies", &["Companies people work at."]),
    ("meeting_attendees", &["Who attended which meeting."]),
    ("calendar_accounts", &["Connected calendar accounts."]),
    (
        "calendar_events",
        &["Events synced from calendar accounts."],
    ),
    (
        "calendars",
        &["The calendars inside each calendar account (ORG-01)."],
    ),
    (
        "calendar_series_choices",
        &["The user's auto-record choice per recurring series (ORG-02)."],
    ),
    ("folders", &["Folders, nested through `parent_id`."]),
    ("meeting_folders", &["Which meetings sit in which folders."]),
    ("folder_rules", &["Folders' auto-add rules (ORG-05)."]),
    ("tags", &["Tags."]),
    (
        "meeting_tags",
        &["Which tags are applied to which meetings."],
    ),
    (
        "action_items",
        &["Action items, from meetings or standalone."],
    ),
    (
        "action_item_views",
        &["Saved views (filters) of the Action Items page (AI-05)."],
    ),
    (
        "meeting_views",
        &["Saved views of the Meetings page (ORG-07)."],
    ),
    (
        "meeting_merge_parts",
        &["The meetings a merged meeting was made from, with their recordings (ORG-07)."],
    ),
    (
        "llm_calls",
        &["One row per LLM provider call: usage, never content."],
    ),
    (
        "jobs",
        &["Post-meeting pipeline jobs: one row per meeting and stage (FND-08)."],
    ),
    ("job_runs", &["The attempts of each pipeline job."]),
    (
        "egress_log",
        &["One row per outgoing HTTP request: service, host, size, never content."],
    ),
    (
        "templates",
        &["Note templates, built-in and the user's own (NOT-04)."],
    ),
    (
        "template_sections",
        &["The ordered sections of each template."],
    ),
    (
        "voiceprints",
        &["Confirmed exemplars of people's voices (CAP-04)."],
    ),
    (
        "speaker_suggestions",
        &["\"Is this Alex?\" suggestions for unnamed speakers (CAP-04)."],
    ),
    (
        "speaker_tag_spans",
        &["Who the meeting app showed as the active speaker, and when (CAP-05)."],
    ),
    (
        "integration_connections",
        &["Connected integration accounts, without their tokens (INT-00)."],
    ),
    (
        "integration_connection_scopes",
        &["The scopes each integration connection was granted."],
    ),
    (
        "integration_deliveries",
        &["The delivery log: what Deino sent to which integration (INT-00)."],
    ),
    (
        "search_documents",
        &[
            "The keyword search index's documents (ORG-08), kept in step with",
            "their sources by triggers; its FTS5 shadow tables are not tracked.",
        ],
    ),
    (
        "embeddings",
        &["Embedded chunks of meetings for semantic search (AI-01)."],
    ),
    (
        "vocab_terms",
        &["The user's vocabulary terms and replacement rules (TRN-05)."],
    ),
    (
        "vocab_variants",
        &["What each vocabulary term sounds like when misheard (TRN-05)."],
    ),
    (
        "vocab_corrections",
        &["The originals of segments the vocabulary corrected (TRN-05)."],
    ),
    (
        "insight_defs",
        &["Insights the user defined: what to extract from meetings (AI-06)."],
    ),
    (
        "insight_results",
        &["Each insight's latest run on each meeting, with its answers (AI-06)."],
    ),
    (
        "year_recaps",
        &["Cached year-in-review recaps, one per year (ORG-09)."],
    ),
    (
        "watched_folders",
        &["Folders watched for recordings to import (CAP-11)."],
    ),
    (
        "watched_files",
        &["Files watched folders imported or failed to import (CAP-11)."],
    ),
    (
        "automations",
        &["Automations: conditions and the steps to run (AUT-01)."],
    ),
    (
        "automation_runs",
        &["Each automation's latest run on each meeting (AUT-01)."],
    ),
    (
        "automation_run_steps",
        &["The steps of each automation run, with their results (AUT-01)."],
    ),
    (
        "clips",
        &["Named ranges of meetings' recordings, cut into files (TRN-04)."],
    ),
    (
        "comment_threads",
        &["Comment threads anchored to transcript or notes spans (TRN-06)."],
    ),
    ("comments", &["The comments of each thread (TRN-06)."]),
    (
        "briefs",
        &["Pre-meeting briefs, one per upcoming calendar event (NOT-08)."],
    ),
    (
        "example_notes",
        &[
            "Notes of the `_example` reference feature, which lives in an in-memory",
            "database in debug builds only.",
        ],
    ),
];
