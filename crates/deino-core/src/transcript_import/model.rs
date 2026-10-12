//! A meeting sent as JSON (DEV-03's `POST /v1/meetings`), in Circleback's
//! import shape (C §12.1): a name, tags, notes and a transcript of
//! utterances, each either timed (`timestamp`, seconds) or word by word.

use chrono::{DateTime, Utc};
use deino_domain::Millis;
use serde::Deserialize;

/// The most utterances one import may carry.
pub const MAX_UTTERANCES: usize = 20_000;

/// A meeting to import.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TranscriptImport {
    /// The meeting's title.
    pub name: String,
    /// When it happened; now when left out.
    #[serde(default)]
    pub created_at: Option<DateTime<Utc>>,
    /// Tags by name; a missing tag is created.
    #[serde(default)]
    pub tags: Vec<ImportTag>,
    /// The meeting's notes as Markdown, kept as its AI notes (version 1).
    #[serde(default)]
    pub notes: Option<String>,
    /// What was said, in order.
    pub transcript: Vec<ImportUtterance>,
}

/// A tag to put on the imported meeting.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImportTag {
    /// Its name, matched ignoring case.
    pub name: String,
    /// When it applies; used only when the tag is created.
    #[serde(default)]
    pub description: String,
}

/// One stretch of speech.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum ImportUtterance {
    /// Timed as a whole: it ends where the next one starts.
    Timed(TimedUtterance),
    /// Timed word by word.
    Words(WordUtterance),
}

/// An utterance with one start time.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TimedUtterance {
    /// Who said it.
    pub speaker: String,
    /// What was said.
    pub text: String,
    /// Seconds from the start of the meeting.
    pub timestamp: f64,
    /// Ignored: Deino is English only.
    #[serde(default)]
    pub language: Option<String>,
}

/// An utterance timed word by word.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WordUtterance {
    /// Who said it.
    pub speaker: String,
    /// Its words, in order.
    pub words: Vec<ImportWord>,
}

/// One timed word.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImportWord {
    /// The word.
    pub text: String,
    /// Seconds from the start of the meeting.
    pub start_timestamp: f64,
    /// Seconds from the start of the meeting.
    pub end_timestamp: f64,
}

/// An utterance reduced to what a segment stores.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Line {
    pub(super) speaker: String,
    pub(super) text: String,
    pub(super) start: Millis,
    pub(super) end: Option<Millis>,
    pub(super) words: Vec<(String, Millis, Millis)>,
}

impl ImportUtterance {
    /// The utterance as a line, or why it cannot be one.
    pub(super) fn line(&self) -> Result<Line, &'static str> {
        match self {
            Self::Timed(timed) => Ok(Line {
                speaker: timed.speaker.trim().to_owned(),
                text: timed.text.trim().to_owned(),
                start: millis(timed.timestamp)?,
                end: None,
                words: Vec::new(),
            }),
            Self::Words(spoken) => words_line(spoken),
        }
    }
}

fn words_line(spoken: &WordUtterance) -> Result<Line, &'static str> {
    let mut words = Vec::with_capacity(spoken.words.len());
    for word in &spoken.words {
        let (start, end) = (millis(word.start_timestamp)?, millis(word.end_timestamp)?);
        if end < start {
            return Err("a word ends before it starts");
        }
        words.push((word.text.trim().to_owned(), start, end));
    }
    let (Some(first), Some(last)) = (words.first(), words.last()) else {
        return Err("an utterance has no words");
    };
    let text: Vec<&str> = words.iter().map(|(text, ..)| text.as_str()).collect();
    Ok(Line {
        speaker: spoken.speaker.trim().to_owned(),
        text: text.join(" "),
        start: first.1,
        end: Some(last.2),
        words,
    })
}

/// Seconds as milliseconds, when they are a time inside a meeting.
fn millis(seconds: f64) -> Result<Millis, &'static str> {
    let ms = (seconds * 1_000.0).round();
    if !ms.is_finite() || ms < 0.0 || ms > f64::from(u32::MAX) {
        return Err("a timestamp is negative or too large");
    }
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "checked to be a whole number inside u32's range just above"
    )]
    Ok(Millis(ms as u32))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_utterance_shapes_parse_into_lines() {
        let import: TranscriptImport = serde_json::from_value(serde_json::json!({
            "name": "Sync",
            "transcript": [
                { "speaker": "Ana", "text": " Hello ", "timestamp": 1.5 },
                { "speaker": "Ben", "words": [
                    { "text": "Hi", "startTimestamp": 3.0, "endTimestamp": 3.2 },
                    { "text": "there", "startTimestamp": 3.3, "endTimestamp": 3.6 }
                ] }
            ]
        }))
        .unwrap();

        let lines: Vec<Line> = import.transcript.iter().map(|u| u.line().unwrap()).collect();
        assert_eq!(lines[0].text, "Hello");
        assert_eq!(lines[0].start, Millis(1_500));
        assert_eq!(lines[1].text, "Hi there");
        assert_eq!((lines[1].start, lines[1].end), (Millis(3_000), Some(Millis(3_600))));
    }

    #[test]
    fn negative_or_endless_times_are_refused() {
        assert!(millis(-1.0).is_err());
        assert!(millis(f64::INFINITY).is_err());
        assert!(millis(1e12).is_err());
        assert_eq!(millis(0.0004), Ok(Millis(0)));
    }
}
