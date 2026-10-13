//! [`AutoStopService`]: the rules around the auto-stop policy. Once a tick
//! it reads the recording, the meeting app's hold on the microphone and the
//! calendar, asks [`AutoStopPolicy`] and the back-to-back rule, and acts
//! through [`AutoStopActions`]. The user's answers (Keep recording, Start
//! new note, Not now) come in through its methods. No OS calls, no Tauri.

use std::collections::HashSet;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use chrono::{DateTime, Utc};
use deino_domain::{BundleId, CalendarEventId, Clock, MeetingId};
use serde::Serialize;

use super::boundary::{EventWindow, current_event, next_meeting};
use super::policy::{AutoStopPolicy, Decision, EndingNotice, Heard, Observation};
use super::ports::{
    AutoStopActions, AutoStopSettings, CalendarWindows, MicrophoneHolders, RecordingProbe,
    RecordingView, SpeechTotals,
};

/// How long an armed app waits for its recording to start.
const ARMED_FOR: chrono::TimeDelta = chrono::TimeDelta::minutes(1);

/// "Start new note for …?": the next meeting has begun.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SplitOffer {
    /// The event the new note would be for.
    pub event_id: CalendarEventId,
    /// Its title.
    pub title: String,
    /// When it started.
    pub starts_at: DateTime<Utc>,
}

/// What auto-stop shows now: the "Ending…" notice and the split offer.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AutoStopState {
    /// The recording ends on its own at the notice's time unless kept.
    pub ending: Option<EndingNotice>,
    /// A new note is offered for the meeting that just began.
    pub split: Option<SplitOffer>,
}

/// Everything the service reads and acts through.
pub struct AutoStopPorts {
    /// The live recorder.
    pub recording: Arc<dyn RecordingProbe>,
    /// Who holds the microphone.
    pub microphone: Arc<dyn MicrophoneHolders>,
    /// The calendar around now.
    pub calendar: Arc<dyn CalendarWindows>,
    /// The settings.
    pub settings: Arc<dyn AutoStopSettings>,
    /// Where decisions go.
    pub actions: Arc<dyn AutoStopActions>,
}

/// The recording being followed.
#[derive(Clone, Debug)]
struct Followed {
    meeting: MeetingId,
    /// The meeting app whose release ends the recording.
    app: Option<BundleId>,
    /// The calendar event the recording is for.
    event: Option<EventWindow>,
    speech: SpeechTotals,
}

/// The service's state between ticks.
#[derive(Debug, Default)]
struct State {
    policy: AutoStopPolicy,
    followed: Option<Followed>,
    /// The app the next recording follows (a detected meeting's), and when
    /// it was armed.
    armed: Option<(BundleId, DateTime<Utc>)>,
    offer: Option<EventWindow>,
    /// Events already offered (answered or not): never asked twice.
    offered: HashSet<CalendarEventId>,
    shown: AutoStopState,
}

/// Auto-stop and meeting boundaries.
pub struct AutoStopService {
    ports: AutoStopPorts,
    clock: Arc<dyn Clock>,
    state: Mutex<State>,
}

impl AutoStopService {
    /// A service following no recording yet.
    pub fn new(ports: AutoStopPorts, clock: Arc<dyn Clock>) -> Self {
        Self {
            ports,
            clock,
            state: Mutex::new(State::default()),
        }
    }

    /// The next recording follows `app` (a detected meeting started it).
    pub fn arm(&self, app: BundleId) {
        self.lock().armed = Some((app, self.clock.now()));
    }

    /// What is shown now.
    pub fn state(&self) -> AutoStopState {
        self.lock().shown.clone()
    }

    /// One tick: follow the recording, decide, act.
    pub fn tick(&self) {
        let now = self.clock.now();
        let config = self.ports.settings.config();
        let view = self.ports.recording.recording();
        let mut state = self.lock();
        if view.is_none() {
            state.followed = None;
            state.armed.take_if(|(_, at)| now - *at > ARMED_FOR);
        }
        let observation = view
            .as_ref()
            .map(|view| self.observe(&mut state, view, now))
            .map(|mut observation| {
                if !config.calendar_end {
                    observation.calendar_end = None;
                }
                observation
            });
        let decision = match (&observation, config.enabled) {
            (Some(observation), false) if observation.disk_full => {
                Decision::Stop(super::StopReason::DiskFull)
            }
            (_, false) => state.policy.tick(None, now),
            (observation, true) => state.policy.tick(observation.as_ref(), now),
        };
        if config.back_to_back {
            self.offer_split(&mut state, view.as_ref(), now);
        } else {
            state.offer = None;
        }
        self.act(state, decision);
    }

    /// The Mac is about to sleep.
    pub fn will_sleep(&self) {
        if !self.ports.settings.config().enabled {
            return;
        }
        let mut state = self.lock();
        let decision = state.policy.will_sleep();
        self.act(state, decision);
    }

    /// "Keep recording": withdraws the "Ending…" notice.
    pub fn keep_recording(&self) {
        let mut state = self.lock();
        let decision = state.policy.keep_recording(self.clock.now());
        self.act(state, decision);
    }

    /// "Start new note": ends this recording and starts one for the offer.
    pub fn accept_split(&self) {
        let mut state = self.lock();
        let Some(event) = state.offer.take() else {
            return;
        };
        self.publish(&mut state);
        drop(state);
        self.ports.actions.split(&event);
    }

    /// "Not now": the offer goes away and is not made again.
    pub fn dismiss_split(&self) {
        let mut state = self.lock();
        if state.offer.take().is_some() {
            self.publish(&mut state);
        }
    }

    /// The observation for `view`, following a new meeting when it changed.
    fn observe(&self, state: &mut State, view: &RecordingView, now: DateTime<Utc>) -> Observation {
        let mut followed = match state.followed.take() {
            Some(followed) if followed.meeting == view.meeting => followed,
            _ => {
                state.offer = None;
                let armed = state.armed.take().map(|(app, _)| app);
                self.follow(armed, view, now)
            }
        };
        if followed.app.is_none() {
            // Recording can start before the call: look until detection sees it.
            followed.app = (self.ports.microphone.meeting_app())
                .inspect(|app| log::info!(target: "detect", "auto-stop: following {app}"));
        }
        let heard = speech_since(&followed.speech, &view.speech);
        followed.speech = view.speech;
        let observation = Observation {
            meeting: view.meeting.clone(),
            paused: view.paused,
            heard,
            app_holds_mic: followed
                .app
                .as_ref()
                .map(|app| self.ports.microphone.holds(app)),
            calendar_end: followed.event.as_ref().map(|event| event.ends_at),
            disk_full: view.disk_full,
        };
        state.followed = Some(followed);
        observation
    }

    /// Starts following `view`'s meeting: its app and its calendar event.
    fn follow(
        &self,
        armed: Option<BundleId>,
        view: &RecordingView,
        now: DateTime<Utc>,
    ) -> Followed {
        let events = self.ports.calendar.events_near(now);
        Followed {
            meeting: view.meeting.clone(),
            app: (armed.or_else(|| self.ports.microphone.meeting_app()))
                .inspect(|app| log::info!(target: "detect", "auto-stop: following {app}")),
            event: current_event(&events, view.event, view.started_at),
            speech: view.speech,
        }
    }

    /// Makes (or withdraws) the back-to-back offer.
    fn offer_split(&self, state: &mut State, view: Option<&RecordingView>, now: DateTime<Utc>) {
        let Some(view) = view else {
            state.offer = None;
            return;
        };
        if state.offer.is_some() {
            return;
        }
        let events = self.ports.calendar.events_near(now);
        let current = state.followed.as_ref().and_then(|f| f.event.as_ref());
        let next = next_meeting(now, current, view.started_at, &events)
            .filter(|event| !state.offered.contains(&event.id))
            .cloned();
        if let Some(event) = next {
            state.offered.insert(event.id);
            state.offer = Some(event);
        }
    }

    /// Publishes what changed and carries out the decision, outside the lock.
    fn act(&self, mut state: MutexGuard<'_, State>, decision: Decision) {
        self.publish(&mut state);
        drop(state);
        if decision != Decision::None {
            log::info!(target: "detect", "auto-stop: {decision:?}");
        }
        if let Decision::Stop(reason) = decision {
            self.ports.actions.stop(reason);
        }
    }

    /// Tells the app when the notice or the offer changed.
    fn publish(&self, state: &mut State) {
        let shown = AutoStopState {
            ending: state.policy.ending(),
            split: state.offer.as_ref().map(offer),
        };
        if shown != state.shown {
            state.shown = shown.clone();
            self.ports.actions.changed(&shown);
        }
    }

    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// The offer for `event`.
fn offer(event: &EventWindow) -> SplitOffer {
    SplitOffer {
        event_id: event.id,
        title: event.title.clone(),
        starts_at: event.starts_at,
    }
}

/// Who was heard since `before`.
fn speech_since(before: &SpeechTotals, now: &SpeechTotals) -> Heard {
    Heard {
        mic: now.mic_speaking || now.mic > before.mic,
        system: now.system_speaking || now.system > before.system,
    }
}

#[cfg(test)]
#[path = "service_tests.rs"]
mod tests;
