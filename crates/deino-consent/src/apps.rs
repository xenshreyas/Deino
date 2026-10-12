//! One [`ChatAdapter`] per app: CAP-05's speaker adapter for the call
//! rules, plus the app's chat words. The words were taken from the call
//! windows the fixture dumps model (`deino-speaker-tags/fixtures/*/chat-*`);
//! when an app changes its UI, capture a new dump and update the table.

use deino_domain::CallApp;
use deino_speaker_tags::{MeetAdapter, SpeakerAdapter, TeamsAdapter, ZoomAdapter};

use crate::chat::{ChatAdapter, ChatVocabulary};

/// Zoom Workplace's in-meeting chat: a panel in the "Zoom Meeting" window,
/// opened from the "Chat" control beside "End".
const ZOOM: ChatVocabulary = ChatVocabulary {
    composer: &["type message", "message everyone", "chat message"],
    send: &["send", "send message"],
    toggle: &["chat", "open chat", "show chat"],
    leave: &["end", "leave", "end meeting", "leave meeting"],
};

/// New Teams' meeting chat: a pane in the meeting window, opened from the
/// "Chat" control beside "Leave" (not the app bar's "Chat").
const TEAMS: ChatVocabulary = ChatVocabulary {
    composer: &[
        "type a message",
        "type a new message",
        "type a meeting message",
    ],
    send: &["send", "send message"],
    toggle: &["chat", "show conversation", "open chat"],
    leave: &["leave", "hang up"],
};

/// Meet's "In-call messages" panel, opened from "Chat with everyone".
const MEET: ChatVocabulary = ChatVocabulary {
    composer: &["send a message", "send message"],
    send: &["send a message", "send message", "send"],
    toggle: &["chat with everyone", "chat"],
    leave: &["leave call"],
};

/// Zoom's meeting chat.
#[derive(Clone, Copy, Debug, Default)]
pub struct ZoomChat;

impl ChatAdapter for ZoomChat {
    fn speaker(&self) -> &'static dyn SpeakerAdapter {
        &ZoomAdapter
    }

    fn vocabulary(&self) -> &'static ChatVocabulary {
        &ZOOM
    }
}

/// Teams' meeting chat.
#[derive(Clone, Copy, Debug, Default)]
pub struct TeamsChat;

impl ChatAdapter for TeamsChat {
    fn speaker(&self) -> &'static dyn SpeakerAdapter {
        &TeamsAdapter
    }

    fn vocabulary(&self) -> &'static ChatVocabulary {
        &TEAMS
    }
}

/// Meet's in-call messages, in Chrome.
#[derive(Clone, Copy, Debug, Default)]
pub struct MeetChat;

impl ChatAdapter for MeetChat {
    fn speaker(&self) -> &'static dyn SpeakerAdapter {
        &MeetAdapter
    }

    fn vocabulary(&self) -> &'static ChatVocabulary {
        &MEET
    }
}

/// The chat adapter for `app`.
pub fn chat_adapter_for(app: CallApp) -> &'static dyn ChatAdapter {
    match app {
        CallApp::Zoom => &ZoomChat,
        CallApp::Teams => &TeamsChat,
        CallApp::Meet => &MeetChat,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_app_has_a_chat_adapter_over_its_speaker_adapter() {
        for app in CallApp::ALL {
            assert_eq!(chat_adapter_for(*app).app(), *app);
        }
    }
}
