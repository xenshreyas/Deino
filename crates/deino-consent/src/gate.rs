//! When to post: [`decide`], a pure function of one moment of a meeting.
//!
//! The notice goes out once transcription runs **and** speech has been
//! heard, into a chat composer that is **empty**, and never while the user
//! is typing: no key press for [`QUIET`]. Anything short of that waits for
//! a later poll. Once per meeting is the session's rule
//! ([`crate::session`]); the re-checks between the steps of a post are the
//! attempt's ([`crate::attempt`]).

use std::time::Duration;

use deino_speaker_tags::WindowSummary;

use crate::chat::{ChatView, Composer};
use crate::path::NodePath;

/// How long the keyboard must have been still before Deino touches a chat.
pub const QUIET: Duration = Duration::from_secs(3);

/// Whether the user is not typing: the last key press is at least
/// [`QUIET`] ago. Unknown key activity counts as typing.
pub fn is_quiet(since_last_key: Option<Duration>) -> bool {
    since_last_key.is_some_and(|since| since >= QUIET)
}

/// Why the notice waits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hold {
    /// The recording is paused (or not this meeting's).
    NotTranscribing,
    /// Nothing has been transcribed yet.
    NoSpeech,
    /// No enabled app shows a call.
    NoCall,
    /// The call shows no chat and no way to open it.
    NoChat,
    /// The user pressed a key less than [`QUIET`] ago.
    Typing,
    /// The composer holds the user's own draft.
    Drafting,
    /// The chat has no send button the adapter knows.
    NoSendButton,
}

/// What to do now.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Decision {
    /// Nothing yet.
    Wait(Hold),
    /// Open the chat panel with its button; post on a later poll.
    OpenChat {
        /// The window the button is in.
        window: WindowSummary,
        /// The chat button.
        toggle: NodePath,
    },
    /// Post the notice through this empty composer.
    Post(Composer),
}

/// One moment of a meeting, as the poll saw it.
#[derive(Clone, Copy, Debug)]
pub struct Moment<'a> {
    /// The meeting is being recorded and transcribed (not paused).
    pub transcribing: bool,
    /// Speech has been transcribed.
    pub speech: bool,
    /// Time since the last key press, if known.
    pub since_last_key: Option<Duration>,
    /// What the call window shows of the chat.
    pub chat: &'a ChatView,
}

/// What to do at `moment`.
pub fn decide(moment: &Moment<'_>) -> Decision {
    if !moment.transcribing {
        return Decision::Wait(Hold::NotTranscribing);
    }
    if !moment.speech {
        return Decision::Wait(Hold::NoSpeech);
    }
    if !is_quiet(moment.since_last_key) {
        return Decision::Wait(Hold::Typing);
    }
    match moment.chat {
        ChatView::NotInCall => Decision::Wait(Hold::NoCall),
        ChatView::Hidden => Decision::Wait(Hold::NoChat),
        ChatView::Closed { window, toggle } => Decision::OpenChat {
            window: window.clone(),
            toggle: toggle.clone(),
        },
        ChatView::Open(composer) if composer.draft => Decision::Wait(Hold::Drafting),
        ChatView::Open(composer) if composer.send.is_none() => Decision::Wait(Hold::NoSendButton),
        ChatView::Open(composer) => Decision::Post(composer.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn composer(draft: bool, send: bool) -> ChatView {
        ChatView::Open(Composer {
            window: WindowSummary::default(),
            field: NodePath::new(vec![0]),
            role: "AXTextArea".into(),
            send: send.then(|| NodePath::new(vec![1])),
            draft,
        })
    }

    fn ready(chat: &ChatView) -> Moment<'_> {
        Moment {
            transcribing: true,
            speech: true,
            since_last_key: Some(Duration::from_secs(10)),
            chat,
        }
    }

    #[test]
    fn an_empty_composer_in_a_quiet_transcribed_call_is_posted_to() {
        let chat = composer(false, true);

        assert!(matches!(decide(&ready(&chat)), Decision::Post(_)));
    }

    #[test]
    fn it_waits_for_transcription_and_speech() {
        let chat = composer(false, true);
        let paused = Moment {
            transcribing: false,
            ..ready(&chat)
        };
        let silent = Moment {
            speech: false,
            ..ready(&chat)
        };

        assert_eq!(decide(&paused), Decision::Wait(Hold::NotTranscribing));
        assert_eq!(decide(&silent), Decision::Wait(Hold::NoSpeech));
    }

    #[test]
    fn it_never_acts_while_the_user_is_typing() {
        let chat = composer(false, true);
        let closed = ChatView::Closed {
            window: WindowSummary::default(),
            toggle: NodePath::new(vec![2]),
        };
        for since in [
            None,
            Some(Duration::ZERO),
            Some(Duration::from_millis(2_999)),
        ] {
            for chat in [&chat, &closed] {
                let typing = Moment {
                    since_last_key: since,
                    ..ready(chat)
                };
                assert_eq!(decide(&typing), Decision::Wait(Hold::Typing));
            }
        }
        let just_quiet = Moment {
            since_last_key: Some(QUIET),
            ..ready(&chat)
        };
        assert!(matches!(decide(&just_quiet), Decision::Post(_)));
    }

    #[test]
    fn a_draft_or_a_missing_send_button_waits() {
        let draft = composer(true, true);
        let no_send = composer(false, false);

        assert_eq!(decide(&ready(&draft)), Decision::Wait(Hold::Drafting));
        assert_eq!(decide(&ready(&no_send)), Decision::Wait(Hold::NoSendButton));
    }

    #[test]
    fn a_closed_chat_is_opened_and_no_call_waits() {
        let closed = ChatView::Closed {
            window: WindowSummary::default(),
            toggle: NodePath::new(vec![2]),
        };

        assert!(matches!(decide(&ready(&closed)), Decision::OpenChat { .. }));
        assert_eq!(
            decide(&ready(&ChatView::NotInCall)),
            Decision::Wait(Hold::NoCall)
        );
        assert_eq!(
            decide(&ready(&ChatView::Hidden)),
            Decision::Wait(Hold::NoChat)
        );
    }
}
