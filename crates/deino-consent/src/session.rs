//! [`Session`]: the notice for one meeting, polled until it is posted or
//! given up. **One send per meeting**: once [`Progress`] is final, a session
//! never acts again, and the [`Monitor`](crate::Monitor) never opens a second
//! session for the same meeting.
//!
//! Each poll finds the meeting chat in the enabled apps' call windows (the
//! CAP-05 window reader), asks [`decide`] what to do, and does at most one
//! thing: open the chat panel, or one [`post`] attempt. Driver failures
//! retry on later polls up to [`MAX_FAILURES`]; every error is logged once.

use std::collections::HashSet;

use deino_domain::{CallApp, MeetingId};
use deino_speaker_tags::{Pid, WindowSummary, tree::BUTTON_ROLE};

use crate::apps::chat_adapter_for;
use crate::attempt::{Outcome, Post, Retry, post};
use crate::chat::{ChatView, Composer};
use crate::error::{ChatError, LoggedOnce};
use crate::gate::{Decision, Hold, Moment, decide};
use crate::path::NodePath;
use crate::ports::{ChatConfig, ConsentPlatform, Recorder, Target};

/// Driver failures tolerated per meeting before the notice gives up.
pub const MAX_FAILURES: u8 = 3;

/// Where a meeting's notice stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Progress {
    /// Not posted yet.
    Waiting,
    /// Posted.
    Sent,
    /// Not posted, and never will be for this meeting.
    GaveUp,
}

impl Progress {
    /// Whether the session is over.
    pub fn is_final(self) -> bool {
        self != Self::Waiting
    }
}

/// The ports a poll uses.
#[derive(Clone, Copy)]
pub struct Ports<'a> {
    /// Accessibility, the driver and the keyboard.
    pub platform: &'a ConsentPlatform,
    /// The recorder.
    pub recorder: &'a dyn Recorder,
}

/// The chat found in one window of one app.
#[derive(Clone, Debug)]
struct Found {
    app: CallApp,
    pid: Pid,
    view: ChatView,
}

/// Where a decision is carried out: the ports, the chat found, the message.
struct Scene<'a> {
    platform: &'a ConsentPlatform,
    found: &'a Found,
    message: &'a str,
}

/// One meeting's notice.
#[derive(Debug)]
pub struct Session {
    meeting: MeetingId,
    progress: Progress,
    failures: u8,
    speech: bool,
    prepared: HashSet<Pid>,
    logged: LoggedOnce,
}

impl Session {
    /// A session for `meeting`; nothing posted yet.
    pub fn new(meeting: MeetingId) -> Self {
        Self {
            meeting,
            progress: Progress::Waiting,
            failures: 0,
            speech: false,
            prepared: HashSet::new(),
            logged: LoggedOnce::default(),
        }
    }

    /// The meeting.
    pub fn meeting(&self) -> &MeetingId {
        &self.meeting
    }

    /// Where the notice stands.
    pub fn progress(&self) -> Progress {
        self.progress
    }

    /// One poll: `transcribing` is false while the recording is paused.
    pub fn tick(&mut self, ports: Ports<'_>, config: &ChatConfig, transcribing: bool) -> Progress {
        if self.progress.is_final() || !config.is_on() {
            return self.progress;
        }
        if !ports.platform.tags.permission.granted() {
            self.logged.log(&ChatError::PermissionDenied);
            return self.progress;
        }
        self.speech = self.speech || (transcribing && ports.recorder.heard_speech(&self.meeting));
        let found = self.find(ports.platform, &config.apps);
        let view = found
            .as_ref()
            .map_or(&ChatView::NotInCall, |found| &found.view);
        let decision = decide(&Moment {
            transcribing,
            speech: self.speech,
            since_last_key: ports.platform.keys.since_last_key(),
            chat: view,
        });
        if let Some(found) = &found {
            let scene = Scene {
                platform: ports.platform,
                found,
                message: &config.message,
            };
            self.act(&scene, decision);
        }
        self.progress
    }

    /// Does what `decision` says in the scene's app.
    fn act(&mut self, scene: &Scene<'_>, decision: Decision) {
        let found = scene.found;
        match decision {
            Decision::Wait(Hold::NoSendButton) => {
                self.logged.log(&ChatError::NoSendButton(found.app));
            }
            Decision::Wait(_) => {}
            Decision::OpenChat { window, toggle } => {
                let button = target(found, &window, toggle, BUTTON_ROLE);
                if let Err(error) = scene.platform.driver.press(&button) {
                    self.fail(&error);
                }
            }
            Decision::Post(composer) => self.post(scene, &composer),
        }
    }

    /// One attempt to post through `composer`.
    fn post(&mut self, scene: &Scene<'_>, composer: &Composer) {
        let (platform, found) = (scene.platform, scene.found);
        let Some(send) = &composer.send else {
            return;
        };
        let outcome = post(&Post {
            driver: platform.driver.as_ref(),
            keys: platform.keys.as_ref(),
            field: target(
                found,
                &composer.window,
                composer.field.clone(),
                &composer.role,
            ),
            send: target(found, &composer.window, send.clone(), BUTTON_ROLE),
            message: scene.message,
        });
        match outcome {
            Outcome::Sent => {
                log::info!(target: "deino_consent", "posted the chat notice in {}", found.app);
                self.progress = Progress::Sent;
            }
            Outcome::Retry(Retry::Hold(_)) => {}
            Outcome::Retry(Retry::Failed(error)) => self.fail(&error),
            Outcome::GaveUp(error) => {
                self.logged.log(&error);
                self.progress = Progress::GaveUp;
            }
        }
    }

    /// Counts a driver failure; gives up after [`MAX_FAILURES`].
    fn fail(&mut self, error: &ChatError) {
        self.logged.log(error);
        self.failures = self.failures.saturating_add(1);
        if self.failures >= MAX_FAILURES {
            self.progress = Progress::GaveUp;
        }
    }

    /// The meeting chat in the first enabled app that shows one: an open
    /// composer wins over a closed panel, which wins over a bare call.
    fn find(&mut self, platform: &ConsentPlatform, apps: &[CallApp]) -> Option<Found> {
        let mut best: Option<Found> = None;
        for app in apps {
            for found in self.views(platform, *app) {
                if rank(&found.view) > best.as_ref().map_or(0, |best| rank(&best.view)) {
                    best = Some(found);
                }
            }
        }
        best
    }

    /// What every call window of `app` shows of the chat.
    fn views(&mut self, platform: &ConsentPlatform, app: CallApp) -> Vec<Found> {
        let adapter = chat_adapter_for(app);
        let speaker = adapter.speaker();
        let mut found = Vec::new();
        for pid in platform.tags.apps.pids(speaker.bundle_ids()) {
            if self.prepared.insert(pid) {
                platform.tags.windows.prepare(app, pid);
            }
            match platform
                .tags
                .windows
                .windows(pid, &|window| speaker.is_call_window(window))
            {
                Ok(windows) => found.extend(windows.iter().map(|window| Found {
                    app,
                    pid,
                    view: adapter.view(window),
                })),
                Err(source) => {
                    self.logged.log(&ChatError::Ax { app, source });
                }
            }
        }
        found
    }
}

/// How far along a view is towards a post.
fn rank(view: &ChatView) -> u8 {
    match view {
        ChatView::NotInCall => 0,
        ChatView::Hidden => 1,
        ChatView::Closed { .. } => 2,
        ChatView::Open(_) => 3,
    }
}

/// The element at `path` in `found`'s window, which must keep `role`.
fn target(found: &Found, window: &WindowSummary, path: NodePath, role: &str) -> Target {
    Target {
        app: found.app,
        pid: found.pid,
        window: window.clone(),
        path,
        role: role.to_owned(),
    }
}
