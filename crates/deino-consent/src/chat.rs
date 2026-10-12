//! [`ChatAdapter`]: where a meeting app's call window keeps its chat, read
//! from the same [`AxNode`] snapshot CAP-05's [`SpeakerAdapter`] reads.
//!
//! An adapter is the app's [`SpeakerAdapter`] (bundle ids, which window is a
//! call, whether a call is on) plus a [`ChatVocabulary`]: the labels of the
//! chat composer, its send button, the button that opens the chat panel
//! and the button that leaves the call. [`view`] turns a window into a
//! [`ChatView`]; it is pure, so the fixture dumps pin every rule.
//!
//! Only a composer inside a **call** window counts: Teams' and Zoom's main
//! windows have composers for ordinary chats, and a notice must never land
//! in one of those.

use deino_domain::CallApp;
use deino_speaker_tags::{AxNode, ReadError, SpeakerAdapter, WindowSummary};

use crate::path::{NodePath, find_first, join};

/// The roles a chat composer has (a native text view, a `textarea`, or a
/// `contenteditable` with `role="textbox"`).
const COMPOSER_ROLES: &[&str] = &["AXTextArea", "AXTextField"];

/// How far above the composer its send button may sit.
const SEND_SEARCH_LEVELS: usize = 3;

/// One app's words for its chat (compared ignoring case).
#[derive(Clone, Copy, Debug)]
pub struct ChatVocabulary {
    /// The composer's label (its `aria-label` or placeholder) starts with one of these.
    pub composer: &'static [&'static str],
    /// The send button's label is one of these.
    pub send: &'static [&'static str],
    /// The button that shows the chat panel is labelled one of these.
    pub toggle: &'static [&'static str],
    /// The call's leave button: the chat toggle is looked for beside it.
    pub leave: &'static [&'static str],
}

/// A chat composer found in a call window.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Composer {
    /// The window it is in.
    pub window: WindowSummary,
    /// The composer, under the window.
    pub field: NodePath,
    /// The composer's role (`AXTextArea` or `AXTextField`).
    pub role: String,
    /// Its send button, under the window, if one was found.
    pub send: Option<NodePath>,
    /// The composer already holds text: the user is writing a message.
    pub draft: bool,
}

/// What a window shows of the meeting chat.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChatView {
    /// The window holds no call.
    NotInCall,
    /// A call without a chat composer or a button to show one.
    Hidden,
    /// The chat panel is closed; the button under the window opens it.
    Closed {
        /// The window the button is in.
        window: WindowSummary,
        /// The chat button.
        toggle: NodePath,
    },
    /// The chat panel is open.
    Open(Composer),
}

/// Finds the meeting chat in one app's call windows.
pub trait ChatAdapter: Send + Sync {
    /// The app's speaker adapter: its bundle ids and call-window rules.
    fn speaker(&self) -> &'static dyn SpeakerAdapter;

    /// The app's chat words.
    fn vocabulary(&self) -> &'static ChatVocabulary;

    /// The app.
    fn app(&self) -> CallApp {
        self.speaker().app()
    }

    /// What `window` shows of the meeting chat.
    fn view(&self, window: &AxNode) -> ChatView {
        view(window, self.speaker(), self.vocabulary())
    }
}

/// What `window` shows of the meeting chat, by `speaker`'s call rules and
/// `words`.
pub fn view(window: &AxNode, speaker: &dyn SpeakerAdapter, words: &ChatVocabulary) -> ChatView {
    let summary = window.summary();
    if !speaker.is_call_window(&summary) || speaker.read(window) == Err(ReadError::NotInCall) {
        return ChatView::NotInCall;
    }
    if let Some(field) = find_first(window, &|node| is_composer(node, words)) {
        let node = field.resolve(window);
        let send = send_button(window, &field, words);
        return ChatView::Open(Composer {
            window: summary,
            role: node.map(|node| node.role.clone()).unwrap_or_default(),
            draft: node.is_some_and(has_draft),
            field,
            send,
        });
    }
    match chat_toggle(window, words) {
        Some(toggle) => ChatView::Closed {
            window: summary,
            toggle,
        },
        None => ChatView::Hidden,
    }
}

/// A composer: a text role whose label starts with the app's composer words.
fn is_composer(node: &AxNode, words: &ChatVocabulary) -> bool {
    COMPOSER_ROLES.contains(&node.role.as_str())
        && label(node).is_some_and(|label| {
            let label = label.to_lowercase();
            words.composer.iter().any(|word| label.starts_with(word))
        })
}

/// What labels a field: its title or `aria-label`, never its value (the
/// text typed into it).
fn label(node: &AxNode) -> Option<&str> {
    [&node.title, &node.description]
        .into_iter()
        .filter_map(Option::as_deref)
        .map(str::trim)
        .find(|text| !text.is_empty())
}

/// Whether the composer holds text other than its own placeholder.
fn has_draft(node: &AxNode) -> bool {
    let value = node.value.as_deref().map_or("", str::trim);
    !value.is_empty() && Some(value) != label(node)
}

/// A button whose label, without a shortcut hint ("Send (Enter)"), is one of `labels`.
fn is_button(node: &AxNode, labels: &[&str]) -> bool {
    node.is(deino_speaker_tags::tree::BUTTON_ROLE)
        && node.text().is_some_and(|text| {
            let main = text.split_once(" (").map_or(text, |(main, _)| main).trim();
            labels.iter().any(|label| main.eq_ignore_ascii_case(label))
        })
}

/// The send button nearest the composer: under its parent, then its
/// grandparent, up to [`SEND_SEARCH_LEVELS`] levels.
fn send_button(window: &AxNode, field: &NodePath, words: &ChatVocabulary) -> Option<NodePath> {
    let mut base = field.parent()?;
    for _ in 0..SEND_SEARCH_LEVELS {
        let scope = base.resolve(window)?;
        if let Some(found) = find_first(scope, &|node| is_button(node, words.send)) {
            return Some(join(&base, &found));
        }
        base = base.parent()?;
    }
    None
}

/// The chat button among the call's controls: beside the leave button,
/// which tells it from an app-wide "Chat" button (Teams' app bar).
fn chat_toggle(window: &AxNode, words: &ChatVocabulary) -> Option<NodePath> {
    let controls = find_first(window, &|node| is_button(node, words.leave))?.parent()?;
    let found = find_first(controls.resolve(window)?, &|node| {
        is_button(node, words.toggle)
    })?;
    Some(join(&controls, &found))
}
