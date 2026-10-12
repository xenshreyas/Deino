//! The consent section (CAP-13): the notice Deino posts in the meeting chat
//! and the reminder card shown when a recording starts.

use serde::{Deserialize, Serialize};

use crate::settings::SettingsError;

/// The longest chat notice, in characters (meeting chats cap messages
/// near this, and a notice should be a line or two).
pub const MAX_CHAT_MESSAGE_CHARS: usize = 500;

/// The notice posted when the user turns an app on without changing it.
pub const DEFAULT_CHAT_MESSAGE: &str = "Hi all, I'm transcribing this call with Deino for my own notes. \
     It runs on my Mac and nothing is shared. Let me know if you'd rather I didn't.";

/// The consent section of `Settings`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct ConsentSettings {
    /// The automatic notice in the meeting chat. Off for every app by default.
    pub chat_message: ChatMessageSettings,
    /// Show the consent reminder card when a recording starts. On by
    /// default; "Don't show again" turns it off.
    pub reminder: bool,
}

impl Default for ConsentSettings {
    fn default() -> Self {
        Self {
            chat_message: ChatMessageSettings::default(),
            reminder: true,
        }
    }
}

impl ConsentSettings {
    /// Checks the notice.
    pub(super) fn validate(&self) -> Result<(), SettingsError> {
        self.chat_message.validate()
    }
}

/// The automatic notice: one switch per meeting app, and the message.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct ChatMessageSettings {
    /// Post in the Zoom desktop app's meeting chat.
    pub zoom: bool,
    /// Post in the Microsoft Teams desktop app's meeting chat.
    pub teams: bool,
    /// Post in Google Meet's in-call messages, in Chrome.
    pub meet: bool,
    /// What is posted, once per meeting.
    pub message: String,
}

impl Default for ChatMessageSettings {
    fn default() -> Self {
        Self {
            zoom: false,
            teams: false,
            meet: false,
            message: DEFAULT_CHAT_MESSAGE.to_owned(),
        }
    }
}

impl ChatMessageSettings {
    /// Whether any app is switched on.
    pub fn any_on(&self) -> bool {
        self.zoom || self.teams || self.meet
    }

    /// A message of at most [`MAX_CHAT_MESSAGE_CHARS`], and not blank while
    /// an app is on.
    fn validate(&self) -> Result<(), SettingsError> {
        if self.message.chars().count() > MAX_CHAT_MESSAGE_CHARS {
            return Err(SettingsError::invalid(
                "consent.chatMessage.message",
                "must be at most 500 characters",
            ));
        }
        if self.any_on() && self.message.trim().is_empty() {
            return Err(SettingsError::invalid(
                "consent.chatMessage.message",
                "must not be empty while an app is on",
            ));
        }
        Ok(())
    }
}

crate::settings::register_section! {
    /// The meeting-chat notice and the consent reminder (CAP-13).
    consent: ConsentSettings,
    order: 80,
    validate: ConsentSettings::validate,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_notice_is_off_and_the_reminder_on_by_default() {
        let settings = ConsentSettings::default();

        assert!(!settings.chat_message.any_on());
        assert!(settings.reminder);
        assert!(!settings.chat_message.message.is_empty());
        assert!(settings.validate().is_ok());
    }

    #[test]
    fn a_blank_message_is_rejected_only_while_an_app_is_on() {
        let mut chat = ChatMessageSettings {
            message: "  ".into(),
            ..ChatMessageSettings::default()
        };
        assert!(chat.validate().is_ok());
        chat.meet = true;
        assert!(chat.validate().is_err());
    }

    #[test]
    fn an_overlong_message_is_rejected() {
        let chat = ChatMessageSettings {
            message: "a".repeat(MAX_CHAT_MESSAGE_CHARS + 1),
            ..ChatMessageSettings::default()
        };

        assert!(chat.validate().is_err());
    }

    #[test]
    fn a_partial_section_keeps_the_other_defaults() {
        let settings: ConsentSettings =
            serde_json::from_str(r#"{ "chatMessage": { "zoom": true } }"#).unwrap();

        assert!(settings.chat_message.zoom);
        assert!(!settings.chat_message.teams);
        assert_eq!(settings.chat_message.message, DEFAULT_CHAT_MESSAGE);
        assert!(settings.reminder);
    }
}
