//! The link types: [`DeepLink`], [`Navigation`] and [`Intent`].

use deino_domain::{CalendarEventId, MeetingId, Millis};
use serde::Serialize;

/// The URL scheme Deino registers with macOS.
pub const SCHEME: &str = "deino";

/// The longest link Deino reads, in characters. A recipe's data is the only
/// long part; anything longer is refused before it is parsed.
pub const MAX_LINK_CHARS: usize = 64 * 1024;

/// One validated `deino://` link. Build it with `DeepLink::parse`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DeepLink {
    /// `meeting/<id>[?t=<ms>]` (or the export's `meetings/<id>`).
    Meeting {
        /// The meeting to open.
        id: MeetingId,
        /// A moment of its recording, from its start.
        at: Option<Millis>,
    },
    /// `new-note[?event=<id>]`.
    NewNote {
        /// The calendar event the note is for.
        event: Option<CalendarEventId>,
    },
    /// `record/start`.
    RecordStart,
    /// `record/stop`.
    RecordStop,
    /// `search?q=<query>`: the query, trimmed and not empty.
    Search {
        /// What to search for.
        query: String,
    },
    /// `recipe?data=<recipe>`: a shared recipe, not empty.
    Recipe {
        /// The recipe as the link carried it.
        data: String,
    },
}

impl DeepLink {
    /// The route's name, for logs: never an ID, a query or a recipe.
    pub fn route(&self) -> &'static str {
        match self {
            Self::Meeting { .. } => "meeting",
            Self::NewNote { .. } => "new-note",
            Self::RecordStart => "record/start",
            Self::RecordStop => "record/stop",
            Self::Search { .. } => "search",
            Self::Recipe { .. } => "recipe",
        }
    }
}

/// Where the window goes: what the UI takes from [`super::LinkService`] once
/// Deino is unlocked.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Navigation {
    /// A meeting's page, on the transcript at `at_ms` when given.
    #[serde(rename_all = "camelCase")]
    Meeting {
        /// The meeting.
        meeting_id: MeetingId,
        /// A moment of its recording, in milliseconds.
        at_ms: Option<Millis>,
    },
    /// A new note: the note screen, or the event's own note.
    #[serde(rename_all = "camelCase")]
    NewNote {
        /// The calendar event the note is for.
        event_id: Option<CalendarEventId>,
    },
    /// The search page for `query`.
    Search {
        /// What to search for.
        query: String,
    },
    /// A recipe to import. Recipes is not built yet, so the UI says "WIP".
    Recipe,
    /// The meeting's Ask panel when a meeting is open; otherwise just the app.
    Ask,
}

/// What a link or a hotkey asks Deino to do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {
    /// Start recording a new meeting.
    StartRecording,
    /// Stop the recording.
    StopRecording,
    /// Stop the recording if one runs, otherwise start one (the hotkey).
    ToggleRecording,
    /// Bring the window forward and go somewhere.
    Show(Navigation),
}

impl From<DeepLink> for Intent {
    fn from(link: DeepLink) -> Self {
        match link {
            DeepLink::Meeting { id, at } => Self::Show(Navigation::Meeting {
                meeting_id: id,
                at_ms: at,
            }),
            DeepLink::NewNote { event } => Self::Show(Navigation::NewNote { event_id: event }),
            DeepLink::RecordStart => Self::StartRecording,
            DeepLink::RecordStop => Self::StopRecording,
            DeepLink::Search { query } => Self::Show(Navigation::Search { query }),
            DeepLink::Recipe { .. } => Self::Show(Navigation::Recipe),
        }
    }
}
