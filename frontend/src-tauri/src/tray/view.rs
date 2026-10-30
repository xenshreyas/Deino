//! What the menu bar shows, as plain data (CAP-10; Granola §2.4's "time
//! left", Circleback §3.5's countdown and today's meetings). Pure, so the
//! wording and the arithmetic are tested without a menu bar.

use chrono::{DateTime, TimeDelta, Utc};
use deino_domain::CalendarEventId;

/// How soon a meeting must start for the menu bar to count down to it.
pub const COUNTDOWN_FROM: TimeDelta = TimeDelta::minutes(60);

/// How long after its start a meeting still reads as "now".
pub const NOW_FOR: TimeDelta = TimeDelta::minutes(10);

/// The longest meeting title shown next to the menu bar icon.
pub const TITLE_CHARS: usize = 22;

/// A meeting on today's calendar.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Meeting {
    /// The event.
    pub id: CalendarEventId,
    /// Its title (may be empty).
    pub title: String,
    /// When it starts and ends.
    pub starts_at: DateTime<Utc>,
    /// When it ends.
    pub ends_at: DateTime<Utc>,
    /// The call link, if any.
    pub join_url: Option<String>,
}

/// The recording, as the menu bar shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Recording {
    /// Its title.
    pub title: String,
    /// Capture is paused.
    pub paused: bool,
    /// When its clock counts from.
    pub started_at: DateTime<Utc>,
    /// When its calendar event ends, if it has one.
    pub ends_at: Option<DateTime<Utc>>,
}

/// The dot on the menu bar icon while a recording is on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dot {
    /// Red and breathing: capturing.
    Recording,
    /// Amber and still: paused.
    Paused,
}

/// The dot for `recording`, if one is on.
pub fn dot(recording: Option<&Recording>) -> Option<Dot> {
    recording.map(|recording| {
        if recording.paused {
            Dot::Paused
        } else {
            Dot::Recording
        }
    })
}

/// The text next to the menu bar icon at `now`, if any: the countdown to
/// the next meeting today. None while recording: the dot says so instead.
pub fn title(
    recording: Option<&Recording>,
    today: &[Meeting],
    now: DateTime<Utc>,
) -> Option<String> {
    if recording.is_some() {
        return None;
    }
    if let Some(live) = today
        .iter()
        .find(|meeting| meeting.starts_at <= now && now - meeting.starts_at < NOW_FOR)
    {
        return Some(format!("{} · now", short(&live.title)));
    }
    let next = today
        .iter()
        .filter(|meeting| meeting.starts_at > now && meeting.starts_at - now <= COUNTDOWN_FROM)
        .min_by_key(|meeting| meeting.starts_at)?;
    Some(format!(
        "{} in {}",
        short(&next.title),
        minutes(next.starts_at - now)
    ))
}

/// The recording's status in the menu: "12m left" while the event lasts,
/// else the time recorded ("47m"), or "Paused".
pub fn status(recording: &Recording, now: DateTime<Utc>) -> String {
    if recording.paused {
        return "Paused".to_owned();
    }
    match recording.ends_at {
        Some(ends_at) if ends_at > now => format!("{} left", minutes(ends_at - now)),
        _ => minutes(now - recording.started_at),
    }
}

/// A span as the menu bar says it: "4m", "1h 05m"; never "0m".
pub fn minutes(span: TimeDelta) -> String {
    let total = (span.num_seconds().max(0) + 59) / 60;
    let total = total.max(1);
    if total < 60 {
        return format!("{total}m");
    }
    format!("{}h {:02}m", total / 60, total % 60)
}

/// A title short enough for the menu bar, "Untitled" when empty.
pub fn short(title: &str) -> String {
    let title = title.trim();
    if title.is_empty() {
        return "Untitled".to_owned();
    }
    if title.chars().count() <= TITLE_CHARS {
        return title.to_owned();
    }
    let cut: String = title.chars().take(TITLE_CHARS - 1).collect();
    format!("{}…", cut.trim_end())
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    fn at(minutes: i64) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 11, 9, 0, 0).unwrap() + TimeDelta::minutes(minutes)
    }

    fn meeting(start: i64, title: &str) -> Meeting {
        Meeting {
            id: "01926b2c-0000-7000-8000-000000000001".parse().unwrap(),
            title: title.to_owned(),
            starts_at: at(start),
            ends_at: at(start + 30),
            join_url: None,
        }
    }

    #[test]
    fn idle_it_counts_down_to_the_next_meeting_within_the_hour() {
        let today = [meeting(90, "Board prep"), meeting(25, "Design review")];

        assert_eq!(
            title(None, &today, at(0)).as_deref(),
            Some("Design review in 25m")
        );
        assert_eq!(title(None, &[meeting(90, "Later")], at(0)), None);
        assert_eq!(
            title(None, &today, at(27)).as_deref(),
            Some("Design review · now")
        );
    }

    #[test]
    fn recording_it_shows_a_dot_and_no_title() {
        let mut recording = Recording {
            title: "Standup".into(),
            paused: false,
            started_at: at(0),
            ends_at: Some(at(30)),
        };
        let today = [meeting(25, "Design review")];

        assert_eq!(title(Some(&recording), &today, at(18)), None);
        assert_eq!(dot(Some(&recording)), Some(Dot::Recording));
        recording.paused = true;
        assert_eq!(dot(Some(&recording)), Some(Dot::Paused));
        assert_eq!(dot(None), None);
    }

    #[test]
    fn the_menu_shows_the_time_left_in_the_event_or_the_time_recorded() {
        let mut recording = Recording {
            title: "Standup".into(),
            paused: false,
            started_at: at(0),
            ends_at: Some(at(30)),
        };
        assert_eq!(status(&recording, at(18)), "12m left");
        assert_eq!(status(&recording, at(47)), "47m");
        recording.ends_at = None;
        assert_eq!(status(&recording, at(65)), "1h 05m");
        recording.paused = true;
        assert_eq!(status(&recording, at(65)), "Paused");
    }

    #[test]
    fn spans_round_up_to_whole_minutes() {
        assert_eq!(minutes(TimeDelta::seconds(1)), "1m");
        assert_eq!(minutes(TimeDelta::seconds(60)), "1m");
        assert_eq!(minutes(TimeDelta::seconds(61)), "2m");
        assert_eq!(minutes(TimeDelta::zero()), "1m");
    }

    #[test]
    fn long_titles_are_cut_with_an_ellipsis() {
        assert_eq!(short("  "), "Untitled");
        assert_eq!(
            short("Quarterly planning with the platform team"),
            "Quarterly planning wi…"
        );
    }
}
