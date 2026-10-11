//! `deino://` links to a meeting and to a moment in its recording, the
//! shape DEV-05 registers (`deino://meeting/<id>[?t=<ms>]`).

use deino_domain::{MeetingId, Millis};

/// The scheme and path of a meeting link.
const MEETING_PREFIX: &str = "deino://meeting/";

/// The link that opens `meeting`.
pub fn meeting_link(meeting: &MeetingId) -> String {
    format!("{MEETING_PREFIX}{meeting}")
}

/// The link that opens `meeting` at `at` in its recording.
pub fn moment_link(meeting: &MeetingId, at: Millis) -> String {
    format!("{MEETING_PREFIX}{meeting}?t={}", at.get())
}

/// `at` as `m:ss`, or `h:mm:ss` from an hour on.
pub fn clock_time(at: Millis) -> String {
    let seconds = at.get() / 1000;
    let (hours, minutes, seconds) = (seconds / 3600, seconds / 60 % 60, seconds % 60);
    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes}:{seconds:02}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn links_open_the_meeting_or_a_moment_in_it() {
        let id = MeetingId::new("meeting-1");

        assert_eq!(meeting_link(&id), "deino://meeting/meeting-1");
        assert_eq!(
            moment_link(&id, Millis(724_000)),
            "deino://meeting/meeting-1?t=724000"
        );
    }

    #[test]
    fn clock_times_grow_an_hour_field_only_when_needed() {
        assert_eq!(clock_time(Millis(0)), "0:00");
        assert_eq!(clock_time(Millis(724_999)), "12:04");
        assert_eq!(clock_time(Millis(3_723_000)), "1:02:03");
    }
}
