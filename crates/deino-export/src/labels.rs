//! The words and numbers every format writes the same way: dates,
//! durations, transcript timestamps and section headings.

use chrono::{DateTime, FixedOffset, NaiveDate};
use deino_domain::{Millis, Seconds};

use crate::Utterance;

/// Section headings, shared so every format says the same thing.
pub(crate) mod heading {
    /// The attendee list.
    pub const ATTENDEES: &str = "Attendees";
    /// The AI notes.
    pub const NOTES: &str = "Notes";
    /// What the user typed.
    pub const MY_NOTES: &str = "My notes";
    /// The action items.
    pub const ACTION_ITEMS: &str = "Action items";
    /// The insights (AI-06).
    pub const INSIGHTS: &str = "Insights";
    /// The transcript.
    pub const TRANSCRIPT: &str = "Transcript";
}

/// Milliseconds in a second, minute and hour.
const SECOND: u32 = 1_000;
const MINUTE: u32 = 60 * SECOND;
const HOUR: u32 = 60 * MINUTE;

/// "Thursday, October 8, 2026 at 15:31".
pub(crate) fn long_date(at: &DateTime<FixedOffset>) -> String {
    at.format("%A, %B %-d, %Y at %H:%M").to_string()
}

/// "2026-10-08T15:31:04", the local time Obsidian reads as a date property.
pub(crate) fn iso_local(at: &DateTime<FixedOffset>) -> String {
    at.format("%Y-%m-%dT%H:%M:%S").to_string()
}

/// "Oct 9, 2026".
pub(crate) fn short_date(date: NaiveDate) -> String {
    date.format("%b %-d, %Y").to_string()
}

/// "7 min", "1 h 23 min" or "under a minute".
pub(crate) fn duration(length: Seconds) -> String {
    let minutes = length.get() / 60;
    match (minutes / 60, minutes % 60) {
        (0, 0) => "under a minute".to_owned(),
        (0, minutes) => format!("{minutes} min"),
        (hours, 0) => format!("{hours} h"),
        (hours, minutes) => format!("{hours} h {minutes} min"),
    }
}

/// How transcript timestamps read: `mm:ss` for meetings under an hour,
/// `h:mm:ss` beyond, so every line of one transcript lines up.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Clock {
    Minutes,
    Hours,
}

impl Clock {
    /// The style that fits the transcript's last timestamp.
    pub(crate) fn for_transcript(transcript: &[Utterance]) -> Self {
        let last = transcript.iter().map(|utterance| utterance.start).max();
        if last.is_some_and(|start| start.get() >= HOUR) {
            Self::Hours
        } else {
            Self::Minutes
        }
    }

    /// `at` as "04:07" or "1:04:07".
    pub(crate) fn format(self, at: Millis) -> String {
        let (hours, minutes, seconds, _) = split(at);
        match self {
            Self::Minutes => format!("{:02}:{seconds:02}", hours * 60 + minutes),
            Self::Hours => format!("{hours}:{minutes:02}:{seconds:02}"),
        }
    }
}

/// One moment on its own, as "04:31", or "1:04:31" from the first hour on.
pub(crate) fn moment(at: Millis) -> String {
    let clock = if at.get() >= HOUR {
        Clock::Hours
    } else {
        Clock::Minutes
    };
    clock.format(at)
}

/// `at` as a subtitle timestamp: "00:04:07,250" (`separator` ',') for SRT
/// or "00:04:07.250" ('.') for WebVTT.
pub(crate) fn subtitle_time(at: Millis, separator: char) -> String {
    let (hours, minutes, seconds, millis) = split(at);
    format!("{hours:02}:{minutes:02}:{seconds:02}{separator}{millis:03}")
}

fn split(at: Millis) -> (u32, u32, u32, u32) {
    let ms = at.get();
    (
        ms / HOUR,
        ms % HOUR / MINUTE,
        ms % MINUTE / SECOND,
        ms % SECOND,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durations_read_in_hours_and_minutes() {
        assert_eq!(duration(Seconds(30)), "under a minute");
        assert_eq!(duration(Seconds(434)), "7 min");
        assert_eq!(duration(Seconds(3_600)), "1 h");
        assert_eq!(duration(Seconds(4_980)), "1 h 23 min");
    }

    #[test]
    fn transcript_clocks_switch_to_hours_past_the_first_hour() {
        assert_eq!(Clock::Minutes.format(Millis(247_250)), "04:07");
        assert_eq!(Clock::Hours.format(Millis(3_847_000)), "1:04:07");
        assert_eq!(moment(Millis(271_000)), "04:31");
        assert_eq!(moment(Millis(3_847_000)), "1:04:07");
    }

    #[test]
    fn subtitle_times_carry_milliseconds() {
        assert_eq!(subtitle_time(Millis(3_847_250), ','), "01:04:07,250");
        assert_eq!(subtitle_time(Millis(696), '.'), "00:00:00.696");
    }
}
