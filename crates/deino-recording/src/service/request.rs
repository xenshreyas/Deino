//! [`StartRequest`]: what a new recording is for.

use deino_core::meetings::MeetingLink;

/// What a new recording is for.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StartRequest {
    /// The meeting's title; a default when missing or blank.
    pub title: Option<String>,
    /// The calendar event (and series) the meeting is recorded for, if any:
    /// a detected call's event, a split to the next event, a menu-bar Record.
    pub link: Option<MeetingLink>,
}

impl StartRequest {
    /// A recording titled `title`, for no particular event.
    pub fn titled(title: Option<String>) -> Self {
        Self { title, link: None }
    }

    /// Whether the caller named the meeting (a non-blank title), so a start
    /// observer keeps that name.
    pub fn is_titled(&self) -> bool {
        self.title
            .as_deref()
            .is_some_and(|title| !title.trim().is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_non_blank_title_names_the_meeting() {
        assert!(StartRequest::titled(Some("Standup".into())).is_titled());
        assert!(!StartRequest::titled(Some("  ".into())).is_titled());
        assert!(!StartRequest::titled(None).is_titled());
    }
}
