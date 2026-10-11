//! What a search asks for and what it finds.

use chrono::{DateTime, Utc};
use deino_domain::{MeetingId, Millis};
use serde::{Deserialize, Serialize};

/// What a hit points at; one variant per row kind of `search_documents`.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type, sqlx::Type,
)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum HitKind {
    /// A meeting's title.
    Meeting,
    /// The user's own notes of a meeting.
    Note,
    /// A meeting's AI summary.
    Summary,
    /// A transcript segment: a moment, with its time in the recording.
    Segment,
    /// An action item.
    ActionItem,
    /// A person.
    Person,
    /// A company.
    Company,
    /// A folder.
    Folder,
}

impl HitKind {
    /// Whether hits of this kind belong to a meeting's content (and so make
    /// the meeting itself a result).
    pub fn is_meeting_content(self) -> bool {
        matches!(
            self,
            Self::Meeting | Self::Note | Self::Summary | Self::Segment
        )
    }
}

/// A matched range of a [`Snippet`], in UTF-16 code units (JavaScript string
/// indices), `start` inclusive and `end` exclusive.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Highlight {
    /// The first highlighted code unit.
    pub start: u32,
    /// One past the last highlighted code unit.
    pub end: u32,
}

/// Text with the ranges that matched the query.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Snippet {
    /// The text, possibly an excerpt with `…` where it was cut.
    pub text: String,
    /// The matched ranges, in order and never overlapping.
    pub highlights: Vec<Highlight>,
}

impl Snippet {
    /// Text with nothing highlighted.
    pub fn plain(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            highlights: Vec::new(),
        }
    }
}

/// One search result.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Hit {
    /// What it points at.
    pub kind: HitKind,
    /// The ID of what it points at: the meeting, segment, action item,
    /// person, company or folder; the meeting for notes and summaries.
    pub ref_id: String,
    /// The meeting it belongs to, if any.
    pub meeting_id: Option<MeetingId>,
    /// Where it starts in the recording (moments).
    pub ts_ms: Option<Millis>,
    /// Where it ends in the recording (semantic transcript windows).
    pub end_ms: Option<Millis>,
    /// Its name: the meeting's title for meeting content, else its own.
    pub label: Snippet,
    /// The matched text around the hit; empty when only the label matched.
    pub snippet: Snippet,
    /// When its meeting happened.
    pub date: Option<DateTime<Utc>>,
    /// Higher is better. BM25 for keyword hits, cosine similarity for
    /// semantic ones, reciprocal-rank fusion when both are combined.
    pub score: f64,
}

/// What a filter chip narrows by.
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    specta::Type,
    sqlx::Type,
)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum FilterKind {
    /// Meetings the person attended.
    Person,
    /// Meetings with someone from the company.
    Company,
    /// Meetings with the tag.
    Tag,
    /// Meetings in the folder or its subfolders.
    Folder,
    /// One meeting.
    Meeting,
}

/// One `@` filter, as the palette shows it.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct FilterChip {
    /// What it narrows by.
    pub kind: FilterKind,
    /// The person, company, tag, folder or meeting ID.
    pub id: String,
    /// Its name.
    pub label: String,
}

/// Narrows a search to meetings. Every filter must hold (AND); an empty
/// filter set searches everything.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SearchFilters {
    /// `@` filters.
    pub chips: Vec<FilterChip>,
    /// Attendee email domains, such as `acme.com`.
    pub domains: Vec<String>,
}

impl SearchFilters {
    /// Whether nothing narrows the search.
    pub fn is_empty(&self) -> bool {
        self.chips.is_empty() && self.domains.is_empty()
    }

    /// The IDs of the chips of `kind`, in order.
    pub fn ids(&self, kind: FilterKind) -> Vec<&str> {
        self.chips
            .iter()
            .filter(|chip| chip.kind == kind)
            .map(|chip| chip.id.as_str())
            .collect()
    }

    /// Adds `chip` unless it is already there.
    pub fn add(&mut self, chip: FilterChip) {
        if !self.chips.contains(&chip) {
            self.chips.push(chip);
        }
    }

    /// Adds `domain` unless it is already there.
    pub fn add_domain(&mut self, domain: String) {
        if !self.domains.contains(&domain) {
            self.domains.push(domain);
        }
    }
}

/// A search from the palette or the results page.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SearchRequest {
    /// What the user typed: words, `"phrases"`, `@mentions` and domains.
    pub text: String,
    /// Filters chosen as chips.
    #[serde(default)]
    pub filters: SearchFilters,
    /// The most hits of each kind (the palette asks for a few, the results
    /// page for more); clamped to [`MAX_HITS_PER_KIND`].
    pub limit: u32,
}

/// The most hits of one kind a search returns.
pub const MAX_HITS_PER_KIND: u32 = 100;

/// Hits grouped the way the palette and the results page show them.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SearchResults {
    /// One hit per meeting whose title, notes, summary or transcript
    /// matched: its best hit, best meeting first.
    pub meetings: Vec<Hit>,
    /// Transcript moments, best first.
    pub moments: Vec<Hit>,
    /// People.
    pub people: Vec<Hit>,
    /// Companies.
    pub companies: Vec<Hit>,
    /// Action items.
    pub action_items: Vec<Hit>,
    /// Folders.
    pub folders: Vec<Hit>,
    /// Every filter that applied: the request's chips plus the `@mentions`
    /// and domains recognised in the text.
    pub filters: SearchFilters,
    /// Whether semantic results were fused in (the embedding model is ready).
    pub semantic: bool,
}
