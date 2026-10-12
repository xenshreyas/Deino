//! Fakes of the ports, for tests here and in other crates (feature
//! `test-support`). Nothing here touches a real app.

use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use deino_domain::MeetingId;
use deino_speaker_tags::fake::FakeMac;

use crate::error::ChatError;
use crate::path::NodePath;
use crate::ports::{
    ChatConfig, ChatDriver, ConsentPlatform, KeyActivity, NoticeConfig, Recorder, RecorderNow,
    Target,
};

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// What the fake driver was asked to do, in order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Act {
    /// `focus` on the element at the path.
    Focus(NodePath),
    /// `set_value` of the text on the element at the path.
    Set(NodePath, String),
    /// `press` on the element at the path.
    Press(NodePath),
}

/// When the fake user types into the composer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Moment {
    /// Right after Deino focuses the composer.
    AfterFocus,
    /// Right after Deino writes the notice.
    AfterWrite,
}

/// A fake chat: element texts by path, a log of every act, and switches
/// for the ways a real app misbehaves.
#[derive(Debug, Default)]
pub struct FakeChat {
    values: Mutex<HashMap<NodePath, String>>,
    acts: Mutex<Vec<Act>>,
    /// Writes are accepted but the text does not change.
    pub ignores_writes: AtomicBool,
    /// Pressing a button leaves the composer as it is (the send failed).
    pub keeps_text_on_press: AtomicBool,
    /// Every call fails as if the window changed.
    pub moved: AtomicBool,
    user_types: Mutex<Option<(Moment, String)>>,
}

impl FakeChat {
    /// An empty chat.
    pub fn new() -> Arc<Self> {
        Arc::default()
    }

    /// Every act so far.
    pub fn acts(&self) -> Vec<Act> {
        lock(&self.acts).clone()
    }

    /// The presses so far.
    pub fn presses(&self) -> usize {
        self.acts()
            .iter()
            .filter(|act| matches!(act, Act::Press(_)))
            .count()
    }

    /// The text of the element at `path` now.
    pub fn text(&self, path: &NodePath) -> String {
        lock(&self.values).get(path).cloned().unwrap_or_default()
    }

    /// Puts `text` in the element at `path`, as the user would.
    pub fn type_text(&self, path: &NodePath, text: &str) {
        lock(&self.values).insert(path.clone(), text.to_owned());
    }

    /// The user types `text` into whatever Deino touches at `moment`.
    pub fn user_types_at(&self, moment: Moment, text: &str) {
        *lock(&self.user_types) = Some((moment, text.to_owned()));
    }

    fn interject(&self, moment: Moment, path: &NodePath) {
        let mut user = lock(&self.user_types);
        if let Some((when, text)) = user.as_ref()
            && *when == moment
        {
            let typed = format!("{}{text}", self.text(path));
            lock(&self.values).insert(path.clone(), typed);
            *user = None;
        }
    }

    fn check(&self, target: &Target) -> Result<(), ChatError> {
        if self.moved.load(Ordering::SeqCst) {
            Err(ChatError::Moved(target.app))
        } else {
            Ok(())
        }
    }
}

impl ChatDriver for FakeChat {
    fn focus(&self, target: &Target) -> Result<(), ChatError> {
        self.check(target)?;
        lock(&self.acts).push(Act::Focus(target.path.clone()));
        self.interject(Moment::AfterFocus, &target.path);
        Ok(())
    }

    fn value(&self, target: &Target) -> Result<String, ChatError> {
        self.check(target)?;
        Ok(self.text(&target.path))
    }

    fn set_value(&self, target: &Target, text: &str) -> Result<(), ChatError> {
        self.check(target)?;
        lock(&self.acts).push(Act::Set(target.path.clone(), text.to_owned()));
        if !self.ignores_writes.load(Ordering::SeqCst) {
            self.type_text(&target.path, text);
        }
        self.interject(Moment::AfterWrite, &target.path);
        Ok(())
    }

    fn press(&self, target: &Target) -> Result<(), ChatError> {
        self.check(target)?;
        lock(&self.acts).push(Act::Press(target.path.clone()));
        if !self.keeps_text_on_press.load(Ordering::SeqCst) {
            lock(&self.values).clear();
        }
        Ok(())
    }
}

/// Key activity from a script: each call takes the next answer, and the
/// last one repeats.
#[derive(Debug)]
pub struct ScriptedKeys(Mutex<VecDeque<Option<Duration>>>);

impl ScriptedKeys {
    /// Answers `script` in order.
    pub fn new(script: impl IntoIterator<Item = Option<Duration>>) -> Arc<Self> {
        Arc::new(Self(Mutex::new(script.into_iter().collect())))
    }

    /// The last key press was long ago, forever.
    pub fn quiet() -> Arc<Self> {
        Self::new([Some(Duration::from_mins(1))])
    }
}

impl KeyActivity for ScriptedKeys {
    fn since_last_key(&self) -> Option<Duration> {
        let mut script = lock(&self.0);
        if script.len() > 1 {
            script.pop_front().flatten()
        } else {
            script.front().copied().flatten()
        }
    }
}

/// A recorder set by the test.
#[derive(Debug)]
pub struct FakeRecorder {
    now: Mutex<RecorderNow>,
    heard: AtomicBool,
}

impl FakeRecorder {
    /// Recording `meeting`, with speech heard when `heard`.
    pub fn recording(meeting: &str, heard: bool) -> Arc<Self> {
        Arc::new(Self {
            now: Mutex::new(RecorderNow::Recording(MeetingId::new(meeting))),
            heard: AtomicBool::new(heard),
        })
    }

    /// Changes what the recorder does.
    pub fn set(&self, now: RecorderNow) {
        *lock(&self.now) = now;
    }

    /// Speech is heard (or not) from now on.
    pub fn hear(&self, heard: bool) {
        self.heard.store(heard, Ordering::SeqCst);
    }
}

impl Recorder for FakeRecorder {
    fn now(&self) -> RecorderNow {
        lock(&self.now).clone()
    }

    fn heard_speech(&self, _meeting: &MeetingId) -> bool {
        self.heard.load(Ordering::SeqCst)
    }
}

/// Settings set by the test.
#[derive(Debug, Default)]
pub struct FixedConfig(Mutex<ChatConfig>);

impl FixedConfig {
    /// `config`, until changed.
    pub fn new(config: ChatConfig) -> Arc<Self> {
        Arc::new(Self(Mutex::new(config)))
    }

    /// Changes the settings.
    pub fn set(&self, config: ChatConfig) {
        *lock(&self.0) = config;
    }
}

impl NoticeConfig for FixedConfig {
    fn chat(&self) -> ChatConfig {
        lock(&self.0).clone()
    }
}

/// The consent ports over a fake Mac, chat and keyboard.
pub fn platform(
    mac: &Arc<FakeMac>,
    chat: &Arc<FakeChat>,
    keys: Arc<ScriptedKeys>,
) -> ConsentPlatform {
    ConsentPlatform {
        tags: mac.platform(),
        driver: chat.clone(),
        keys,
    }
}
