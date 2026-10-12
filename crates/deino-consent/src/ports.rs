//! The ports the chat notice runs on. `macos/` implements the driver and
//! the key activity; CAP-05's [`Platform`] reads the windows; the app
//! implements [`Recorder`] over the recorder and the segments, and
//! [`NoticeConfig`] over the settings; `fake` (feature `test-support`) has
//! one of each for tests.

use std::sync::Arc;
use std::time::Duration;

use deino_domain::{CallApp, MeetingId};
use deino_speaker_tags::{Pid, Platform, WindowSummary};

use crate::error::ChatError;
use crate::path::NodePath;

/// One element of a running app to act on: the window it is in, its path
/// under that window and the role it must still have.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Target {
    /// The app.
    pub app: CallApp,
    /// The app's process.
    pub pid: Pid,
    /// The window, by its title and address (never logged).
    pub window: WindowSummary,
    /// The element under the window.
    pub path: NodePath,
    /// The role the element had when it was read.
    pub role: String,
}

/// Acts on another app's elements through Accessibility. Every call finds
/// the element again from its [`Target`] and fails with
/// [`ChatError::Moved`] when it is gone or has another role.
pub trait ChatDriver: Send + Sync {
    /// Gives the element keyboard focus inside its app (`AXFocused`).
    fn focus(&self, target: &Target) -> Result<(), ChatError>;

    /// The element's text now (`AXValue`); empty when it has none.
    fn value(&self, target: &Target) -> Result<String, ChatError>;

    /// Replaces the element's text (`AXValue`). No key events are posted.
    fn set_value(&self, target: &Target, text: &str) -> Result<(), ChatError>;

    /// Presses a button (`AXPress`).
    fn press(&self, target: &Target) -> Result<(), ChatError>;
}

/// When the user last pressed a key, anywhere.
pub trait KeyActivity: Send + Sync {
    /// Time since the last key press on the keyboard (hardware events
    /// only, so the notice's own writes never count); `None` when unknown.
    fn since_last_key(&self) -> Option<Duration>;
}

/// What the recorder is doing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RecorderNow {
    /// Recording and transcribing `0`.
    Recording(MeetingId),
    /// The recording of `0` is paused.
    Paused(MeetingId),
    /// Nothing is recorded.
    Idle,
}

/// The recorder, as the notice sees it.
pub trait Recorder: Send + Sync {
    /// What the recorder is doing now.
    fn now(&self) -> RecorderNow;

    /// Whether any speech of `meeting` has been transcribed yet.
    fn heard_speech(&self, meeting: &MeetingId) -> bool;
}

/// What the user set for the notice.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ChatConfig {
    /// The message to post.
    pub message: String,
    /// The apps it is posted in; empty when the notice is off.
    pub apps: Vec<CallApp>,
}

impl ChatConfig {
    /// Whether the notice is on for any app, with a message to post.
    pub fn is_on(&self) -> bool {
        !self.apps.is_empty() && !self.message.trim().is_empty()
    }
}

/// The user's settings for the notice, read on every poll.
pub trait NoticeConfig: Send + Sync {
    /// The notice's settings now.
    fn chat(&self) -> ChatConfig;
}

/// The platform's ports, together.
#[derive(Clone)]
pub struct ConsentPlatform {
    /// CAP-05's Accessibility ports: the permission, running apps, windows.
    pub tags: Platform,
    /// Acts on the chat.
    pub driver: Arc<dyn ChatDriver>,
    /// The typing guard's source.
    pub keys: Arc<dyn KeyActivity>,
}
