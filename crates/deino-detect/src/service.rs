//! [`DetectionService`]: the rules around the policy. It turns evidence into
//! prompts, runs the auto-start countdown, answers the prompt's buttons and
//! publishes the [`DetectionEvent`] stream. No OS calls and no Tauri: every
//! effect goes through a port in [`crate::outputs`].

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use chrono::{DateTime, Utc};
use deino_domain::{BundleId, Clock};

use crate::autostop::MicrophoneHolders;

use crate::error::DetectError;
use crate::event::{Detection, DetectionEvent, DetectionId};
use crate::outputs::{
    ConfigSource, DetectionConfig, DetectionSink, IgnoreList, RecordingRequest, RecordingStarter,
    RecordingStatus,
};
use crate::policy::{Answer, DetectionPolicy, Gate, PolicyAction};
use crate::resolver::resolve;
use crate::snapshot::{Candidate, SignalSnapshot};

/// Everything the service acts through.
pub struct DetectionPorts {
    /// Where events go.
    pub sink: Arc<dyn DetectionSink>,
    /// Starts recording.
    pub recorder: Arc<dyn RecordingStarter>,
    /// Whether Deino is recording.
    pub status: Arc<dyn RecordingStatus>,
    /// The user's settings.
    pub config: Arc<dyn ConfigSource>,
    /// Saves "Always ignore this app".
    pub ignore_list: Arc<dyn IgnoreList>,
}

/// Meeting detection's rules, shared by the detector thread and the commands.
pub struct DetectionService {
    ports: DetectionPorts,
    clock: Arc<dyn Clock>,
    state: Mutex<State>,
}

/// The policy, the prompt on screen and what the last evaluation saw.
struct State {
    policy: DetectionPolicy,
    shown: Option<Detection>,
    /// Apps capturing from the microphone at the last evaluation.
    holders: Vec<BundleId>,
    /// The meeting app the last evaluation resolved, if any.
    meeting_app: Option<BundleId>,
}

impl DetectionService {
    /// A service with no prompt showing.
    pub fn new(ports: DetectionPorts, clock: Arc<dyn Clock>) -> Self {
        let policy = DetectionPolicy::new(Arc::clone(&clock));
        Self {
            ports,
            clock,
            state: Mutex::new(State {
                policy,
                shown: None,
                holders: Vec::new(),
                meeting_app: None,
            }),
        }
    }

    /// Evaluates the latest evidence: may show, close or end a prompt, and
    /// starts recording when an auto-start countdown has run out.
    pub fn evaluate(&self, snapshot: &SignalSnapshot) {
        let config = self.ports.config.config();
        let gate = Gate {
            enabled: config.enabled,
            recording: self.ports.status.is_recording(),
        };
        let candidate = resolve(snapshot, &config.ignored);
        let now = self.clock.now();
        let (mut events, auto_start) = {
            let mut state = self.lock();
            state.remember(snapshot, candidate.as_ref());
            let auto_start = state.auto_start_due(now);
            let actions = state.policy.evaluate(candidate, gate);
            (state.apply(actions, &config, now), auto_start)
        };
        if let Some(detection) = &auto_start {
            events.insert(0, DetectionEvent::PromptClosed(detection.id));
        }
        self.publish(events);
        if let Some(detection) = auto_start {
            log::info!(target: "detect", "auto-start countdown ended for prompt {}", detection.id.0);
            self.record(detection, false).unwrap_or_else(
                |error| log::error!(target: "detect", "auto-start failed: {error}"),
            );
        }
    }

    /// The prompt on screen, for a prompt window that opens after the event.
    pub fn current(&self) -> Option<Detection> {
        self.lock().shown.clone()
    }

    /// "Take notes": closes prompt `id` and starts recording its meeting.
    pub fn take_notes(&self, id: DetectionId) -> Result<(), DetectError> {
        let detection = self.answer(id, Answer::TakeNotes)?;
        self.record(detection, true)
    }

    /// "Ignore" or "Don't record": closes prompt `id`; its app stays quiet
    /// for ten minutes.
    pub fn dismiss(&self, id: DetectionId) -> Result<(), DetectError> {
        self.answer(id, Answer::Dismiss).map(drop)
    }

    /// "Always ignore this app": closes prompt `id` and adds its app to the
    /// ignore list.
    pub async fn ignore_app(&self, id: DetectionId) -> Result<(), DetectError> {
        let detection = self.answer(id, Answer::AlwaysIgnore)?;
        self.ports.ignore_list.ignore(&detection.app).await
    }

    /// When the detector must evaluate again even if nothing changes.
    pub fn next_deadline(&self) -> Option<DateTime<Utc>> {
        let state = self.lock();
        let countdown = state.shown.as_ref().and_then(|shown| shown.auto_start_at);
        match (state.policy.next_deadline(), countdown) {
            (Some(policy), Some(countdown)) => Some(policy.min(countdown)),
            (policy, countdown) => policy.or(countdown),
        }
    }

    /// Closes prompt `id` with `answer`, or fails when it already closed.
    fn answer(&self, id: DetectionId, answer: Answer) -> Result<Detection, DetectError> {
        let detection = {
            let mut state = self.lock();
            let shown = state
                .shown
                .take_if(|shown| shown.id == id)
                .ok_or(DetectError::PromptClosed)?;
            state
                .policy
                .answer(id, answer)
                .ok_or(DetectError::PromptClosed)?;
            shown
        };
        self.publish(vec![DetectionEvent::PromptClosed(id)]);
        Ok(detection)
    }

    fn record(&self, detection: Detection, open_notes: bool) -> Result<(), DetectError> {
        self.ports.recorder.start(&RecordingRequest {
            detection,
            open_notes,
        })
    }

    fn publish(&self, events: Vec<DetectionEvent>) {
        for event in events {
            self.ports.sink.publish(event);
        }
    }

    fn lock(&self) -> MutexGuard<'_, State> {
        // The state is replaced whole in each step, so a panic elsewhere
        // cannot leave it half-written.
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl MicrophoneHolders for DetectionService {
    fn holds(&self, app: &BundleId) -> bool {
        self.lock().holders.contains(app)
    }

    fn meeting_app(&self) -> Option<BundleId> {
        self.lock().meeting_app.clone()
    }
}

impl State {
    /// Keeps who held the microphone and which meeting app was resolved,
    /// for auto-stop (CAP-07), which follows a recording's meeting app.
    fn remember(&mut self, snapshot: &SignalSnapshot, candidate: Option<&Candidate>) {
        self.holders = snapshot
            .clients
            .iter()
            .map(|client| client.app.bundle_id.clone())
            .collect();
        self.meeting_app = candidate.map(|candidate| candidate.app.bundle_id.clone());
    }

    /// The shown prompt whose countdown has run out, answered as "Take notes".
    fn auto_start_due(&mut self, now: DateTime<Utc>) -> Option<Detection> {
        let due = self
            .shown
            .as_ref()?
            .auto_start_at
            .is_some_and(|at| at <= now);
        let detection = self.shown.take_if(|_| due)?;
        self.policy.answer(detection.id, Answer::TakeNotes)?;
        Some(detection)
    }

    /// The events for `actions`, keeping `shown` in step with the policy.
    fn apply(
        &mut self,
        actions: Vec<PolicyAction>,
        config: &DetectionConfig,
        now: DateTime<Utc>,
    ) -> Vec<DetectionEvent> {
        actions
            .into_iter()
            .map(|action| match action {
                PolicyAction::Prompt(id, candidate) => {
                    let detection = detection(id, candidate, config, now);
                    self.shown = Some(detection.clone());
                    DetectionEvent::MeetingDetected(detection)
                }
                PolicyAction::Close(id) => {
                    self.shown = None;
                    DetectionEvent::PromptClosed(id)
                }
                PolicyAction::End(id) => {
                    self.shown = None;
                    DetectionEvent::MeetingEnded(id)
                }
            })
            .collect()
    }
}

fn detection(
    id: DetectionId,
    candidate: Candidate,
    config: &DetectionConfig,
    now: DateTime<Utc>,
) -> Detection {
    Detection {
        id,
        kind: candidate.platform.kind(),
        platform: candidate.platform,
        app: candidate.app,
        event: candidate.event,
        detected_at: now,
        auto_start_at: config.auto_start.map(|countdown| now + countdown),
    }
}

#[cfg(test)]
#[path = "service_tests.rs"]
mod tests;
