//! Helpers every adapter maps with: titles, utterances in time order,
//! `HH:MM:SS` stamps, attendees and the meeting itself.

use chrono::{DateTime, Utc};
use deino_core::meetings::MAX_TITLE_CHARS;
use deino_core::transcript_import::{
    ImportActionItem, ImportAttendee, ImportUtterance, TimedUtterance, TranscriptImport,
};

/// The title of a meeting the source left untitled.
pub(crate) const UNTITLED: &str = "Untitled meeting";
/// The speaker of a line the source did not attribute.
pub(crate) const UNKNOWN_SPEAKER: &str = "Speaker";

/// A meeting's parts, before they become a [`TranscriptImport`].
#[derive(Default)]
pub(crate) struct Parts {
    pub(crate) title: Option<String>,
    pub(crate) created_at: Option<DateTime<Utc>>,
    pub(crate) notes: Option<String>,
    pub(crate) transcript: Vec<ImportUtterance>,
    pub(crate) attendees: Vec<ImportAttendee>,
    pub(crate) action_items: Vec<ImportActionItem>,
}

impl Parts {
    /// The import: a usable title, the lines in time order, blank notes dropped.
    pub(crate) fn into_import(self) -> TranscriptImport {
        let mut transcript = self.transcript;
        transcript.sort_by(|a, b| start(a).total_cmp(&start(b)));
        TranscriptImport {
            name: title(self.title.as_deref()),
            created_at: self.created_at,
            tags: Vec::new(),
            notes: self.notes.filter(|notes| !notes.trim().is_empty()),
            transcript,
            attendees: self.attendees,
            action_items: self.action_items,
        }
    }
}

/// `name` trimmed and cut to a meeting title, or [`UNTITLED`].
pub(crate) fn title(name: Option<&str>) -> String {
    let name: String = name
        .unwrap_or_default()
        .trim()
        .chars()
        .take(MAX_TITLE_CHARS)
        .collect();
    if name.trim().is_empty() {
        UNTITLED.to_owned()
    } else {
        name.trim_end().to_owned()
    }
}

/// A timed line, or `None` when it says nothing or its time is unusable.
pub(crate) fn utterance(
    speaker: Option<&str>,
    text: &str,
    start: f64,
    end: Option<f64>,
) -> Option<ImportUtterance> {
    let text = text.trim();
    if text.is_empty() || !start.is_finite() || start < 0.0 {
        return None;
    }
    let speaker = speaker
        .map(str::trim)
        .filter(|speaker| !speaker.is_empty())
        .unwrap_or(UNKNOWN_SPEAKER);
    Some(ImportUtterance::Timed(TimedUtterance {
        speaker: speaker.to_owned(),
        text: text.to_owned(),
        timestamp: start,
        end_timestamp: end.filter(|end| end.is_finite() && *end >= start),
        language: None,
    }))
}

/// An attendee from a name and an address, `None` when both are blank.
pub(crate) fn attendee(name: Option<&str>, email: Option<&str>) -> Option<ImportAttendee> {
    let clean = |value: Option<&str>| {
        value
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
    };
    let (name, email) = (clean(name), clean(email));
    (name.is_some() || email.is_some()).then_some(ImportAttendee {
        name,
        email,
        is_calendar_event_organizer: false,
        is_calendar_invitee: true,
    })
}

/// Seconds in an `HH:MM:SS` (or `MM:SS`) stamp.
pub(crate) fn clock_seconds(stamp: &str) -> Option<f64> {
    stamp
        .trim()
        .split(':')
        .try_fold(0.0, |total, part| Some(total * 60.0 + part.parse::<f64>().ok()?))
}

/// A time sent as milliseconds since the Unix epoch.
pub(crate) fn from_epoch_millis(ms: f64) -> Option<DateTime<Utc>> {
    if !ms.is_finite() || ms.abs() > 1e15 {
        return None;
    }
    #[allow(
        clippy::cast_possible_truncation,
        reason = "finite and well inside i64's range, checked just above"
    )]
    DateTime::from_timestamp_millis(ms.round() as i64)
}

/// An ID the source sends as a string or a number, as text.
pub(crate) fn id_text(id: &serde_json::Value) -> String {
    match id {
        serde_json::Value::String(text) => text.clone(),
        other => other.to_string(),
    }
}

/// Where an utterance starts, for sorting.
fn start(utterance: &ImportUtterance) -> f64 {
    match utterance {
        ImportUtterance::Timed(timed) => timed.timestamp,
        ImportUtterance::Words(spoken) => spoken
            .words
            .first()
            .map_or(0.0, |word| word.start_timestamp),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stamps_titles_and_lines_are_cleaned() {
        assert_eq!(clock_seconds("01:02:03"), Some(3_723.0));
        assert_eq!(clock_seconds("02:03"), Some(123.0));
        assert_eq!(clock_seconds("x"), None);
        assert_eq!(title(Some("  ")), UNTITLED);
        assert!(utterance(None, " ", 1.0, None).is_none());
        assert!(utterance(None, "Hi", -1.0, None).is_none());
        let Some(ImportUtterance::Timed(line)) = utterance(None, " Hi ", 2.0, Some(1.0)) else {
            panic!("a line");
        };
        assert_eq!((line.speaker.as_str(), line.end_timestamp), (UNKNOWN_SPEAKER, None));
    }

    #[test]
    fn lines_are_put_in_time_order() {
        let parts = Parts {
            transcript: vec![
                utterance(Some("B"), "second", 5.0, None).unwrap(),
                utterance(Some("A"), "first", 1.0, None).unwrap(),
            ],
            ..Parts::default()
        };
        let import = parts.into_import();
        assert!((start(&import.transcript[0]) - 1.0).abs() < f64::EPSILON);
        assert_eq!(import.name, UNTITLED);
    }
}
