//! [`AutoTitler`]: names a meeting that has no real name (the `auto_title`
//! stage, route `title`).
//!
//! A meeting keeps a title the user (or a calendar) gave it. When its title
//! is generic ("New note", the recorder's `Meeting 2026-10-08_14-00-00`):
//! a linked calendar event's real title wins without an LLM call; otherwise
//! the `title` route names it from its enhanced notes (short, and already
//! about what mattered), or from the start of its transcript.

use std::sync::{Arc, LazyLock};

use deino_domain::{Clock, MeetingId};
use deino_llm::prompts::{PromptText, TitleNotesUser, TitleSystem, TitleUser};
use deino_llm::{Gateway, Prompt, Route};
use regex::Regex;

use super::EnhanceError;
use super::EnhancedNotesReader;
use super::sources::MaterialSources;
use super::transcript::render_lines;
use crate::meetings::{MAX_TITLE_CHARS, MeetingTitle};

/// Shown when the meeting already has a name.
pub const ALREADY_NAMED: &str = "The meeting already has a title.";
/// Shown when there is nothing to name the meeting from.
pub const NOTHING_TO_NAME: &str = "There is nothing to name the meeting from yet.";
/// Shown when the model found nothing meaningful.
pub const NO_TITLE: &str = "No meaningful title could be written.";

/// Room for a short title.
const TITLE_MAX_TOKENS: u32 = 40;
/// The most of the notes the prompt carries, in characters.
const MAX_NOTES_CHARS: usize = 6_000;
/// Transcript lines read when there are no notes.
const TRANSCRIPT_LINES: usize = 150;
/// The longest title kept, in words.
const MAX_TITLE_WORDS: usize = 12;
/// The model's answer for "nothing meaningful".
const EMPTY_ANSWER: &str = "<EMPTY>";

/// Titles that name nothing: placeholders and the apps' defaults.
const GENERIC_TITLES: [&str; 22] = [
    "",
    "new note",
    "new meeting",
    "+ new call",
    "new call",
    "untitled",
    "untitled meeting",
    "meeting",
    "call",
    "sync",
    "quick sync",
    "quick call",
    "chat",
    "catch up",
    "catch-up",
    "1:1",
    "zoom meeting",
    "google meet",
    "microsoft teams meeting",
    "teams meeting",
    "imported audio",
    "recording",
];

/// The recorder's `Meeting 2026-10-08_14-00-00` and similar date stamps.
static DATED_TITLE: LazyLock<Option<Regex>> = LazyLock::new(|| {
    Regex::new(r"^(meeting|recording|call|note)[\s_-]+\d{4}-\d{2}-\d{2}([\s_t]\d{1,2}[-:.]\d{2}([-:.]\d{2})?)?$").ok()
});

/// True when `title` is a placeholder, not a name.
pub fn is_generic_title(title: &str) -> bool {
    let title = title.trim().to_lowercase();
    GENERIC_TITLES.contains(&title.as_str())
        || DATED_TITLE
            .as_ref()
            .is_some_and(|pattern| pattern.is_match(&title))
}

/// How a run ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TitleRun {
    /// The meeting was renamed.
    Renamed(String),
    /// It kept its title, for this user-safe reason.
    Skipped(&'static str),
}

/// Names meetings that have no real name.
pub struct AutoTitler {
    sources: MaterialSources,
    notes: Arc<dyn EnhancedNotesReader>,
    gateway: Arc<Gateway>,
    clock: Arc<dyn Clock>,
}

impl AutoTitler {
    /// A titler reading through `sources` and `notes`, asking `gateway`.
    pub fn new(
        sources: MaterialSources,
        notes: Arc<dyn EnhancedNotesReader>,
        gateway: Arc<Gateway>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            sources,
            notes,
            gateway,
            clock,
        }
    }

    /// Names `meeting` if its title is generic (see the module docs).
    pub async fn run(&self, meeting: &MeetingId) -> Result<TitleRun, EnhanceError> {
        let material = self
            .sources
            .gather(meeting)
            .await?
            .ok_or_else(|| EnhanceError::MeetingNotFound(meeting.clone()))?;
        if !is_generic_title(&material.meeting.title) {
            return Ok(TitleRun::Skipped(ALREADY_NAMED));
        }
        let calendar = material.meeting.calendar_title.clone();
        let title = match calendar.filter(|title| !is_generic_title(title)) {
            Some(title) => Some(title),
            None => self.generate(meeting, &material.transcript).await?,
        };
        let Some(title) = title.and_then(|title| MeetingTitle::parse(&title).ok()) else {
            return Ok(TitleRun::Skipped(NO_TITLE));
        };
        let renamed = self
            .sources
            .meetings
            .rename(meeting, &title, self.clock.now())
            .await?;
        log::info!(target: "enhance", "{meeting}: titled automatically");
        Ok(
            renamed.map_or(TitleRun::Skipped(NOTHING_TO_NAME), |meeting| {
                TitleRun::Renamed(meeting.title)
            }),
        )
    }

    /// A title from the notes, else the transcript; `None` for nothing.
    async fn generate(
        &self,
        meeting: &MeetingId,
        transcript: &[super::transcript::TranscriptLine],
    ) -> Result<Option<String>, EnhanceError> {
        let notes = self.notes.latest(meeting).await?;
        let user = match notes.filter(|notes| !notes.markdown.trim().is_empty()) {
            Some(notes) => TitleNotesUser {
                notes: &notes
                    .markdown
                    .chars()
                    .take(MAX_NOTES_CHARS)
                    .collect::<String>(),
            }
            .text(),
            None if transcript.is_empty() => return Ok(None),
            None => {
                let start = transcript.get(..TRANSCRIPT_LINES).unwrap_or(transcript);
                TitleUser {
                    transcript: &render_lines(start),
                }
                .text()
            }
        };
        let prompt = Prompt::new(user)
            .system(TitleSystem.text())
            .max_tokens(TITLE_MAX_TOKENS);
        let answer = self.gateway.complete(Route::Title, prompt).await?;
        Ok(clean_title(&answer.text))
    }
}

/// The answer's first line without quotes, Markdown or a final period,
/// at most [`MAX_TITLE_WORDS`] words; `None` for an empty answer.
pub fn clean_title(answer: &str) -> Option<String> {
    let line = answer
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())?;
    let line = line
        .trim_start_matches(['#', '*', '-', ' '])
        .trim_matches(['"', '\'', '“', '”', '*', '`', ' '])
        .trim_end_matches('.');
    if line.is_empty() || line.contains(EMPTY_ANSWER) {
        return None;
    }
    let words: Vec<&str> = line.split_whitespace().take(MAX_TITLE_WORDS).collect();
    let title: String = words.join(" ").chars().take(MAX_TITLE_CHARS).collect();
    Some(title)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholders_and_date_stamps_are_generic() {
        for title in [
            "",
            "New note",
            "  Meeting ",
            "Meeting 2025-10-03_08-25-23",
            "+ New Call",
            "Zoom Meeting",
        ] {
            assert!(is_generic_title(title), "{title:?}");
        }
        for title in [
            "Maya 1:1",
            "Deino × Brightkite: discovery",
            "Q4 planning",
            "Meeting notes for Q4",
        ] {
            assert!(!is_generic_title(title), "{title:?}");
        }
    }

    #[test]
    fn answers_are_cleaned_to_one_short_line() {
        assert_eq!(
            clean_title("\"Q4 roadmap alignment.\"\n").as_deref(),
            Some("Q4 roadmap alignment")
        );
        assert_eq!(
            clean_title("## Brightkite pilot scope").as_deref(),
            Some("Brightkite pilot scope")
        );
        assert_eq!(clean_title("<EMPTY>"), None);
        assert_eq!(clean_title("  \n"), None);
    }
}
