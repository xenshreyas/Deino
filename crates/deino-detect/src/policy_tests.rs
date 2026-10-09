use deino_domain::FakeClock;

use super::*;
use crate::model::Platform;
use crate::testing::{CHROME, PHOTO_BOOTH, ZOOM, app, id, start};

const ON: Gate = Gate {
    enabled: true,
    recording: false,
};
const RECORDING: Gate = Gate {
    enabled: true,
    recording: true,
};
const OFF: Gate = Gate {
    enabled: false,
    recording: false,
};

fn meeting(bundle: &str, dwell: Dwell) -> Candidate {
    Candidate {
        key: CandidateKey::app(&id(bundle)),
        app: app(bundle),
        platform: Platform::Zoom,
        dwell,
        event: None,
    }
}

fn zoom() -> Candidate {
    meeting(ZOOM, Dwell::Short)
}

/// A policy over a fake clock, stepped through time.
struct Harness {
    clock: Arc<FakeClock>,
    policy: DetectionPolicy,
}

impl Harness {
    fn new() -> Self {
        let clock = Arc::new(FakeClock::new(start()));
        Self {
            policy: DetectionPolicy::new(clock.clone()),
            clock,
        }
    }

    /// Advances the clock by `delta`, then evaluates `candidate`.
    fn after(
        &mut self,
        delta: TimeDelta,
        candidate: Option<&Candidate>,
        gate: Gate,
    ) -> Vec<PolicyAction> {
        self.clock.advance(delta);
        self.policy.evaluate(candidate.cloned(), gate)
    }

    /// Shows the Zoom prompt and returns its id.
    fn prompt_zoom(&mut self) -> DetectionId {
        self.after(TimeDelta::zero(), Some(&zoom()), ON);
        match self.after(SHORT_DWELL, Some(&zoom()), ON).as_slice() {
            [PolicyAction::Prompt(id, _)] => *id,
            other => panic!("expected a prompt, got {other:?}"),
        }
    }
}

#[test]
fn a_meeting_app_prompts_once_it_has_held_for_the_short_dwell() {
    let mut harness = Harness::new();

    assert!(
        harness
            .after(TimeDelta::zero(), Some(&zoom()), ON)
            .is_empty()
    );
    assert!(
        harness
            .after(TimeDelta::milliseconds(2_400), Some(&zoom()), ON)
            .is_empty()
    );
    let actions = harness.after(TimeDelta::milliseconds(100), Some(&zoom()), ON);

    assert_eq!(actions, vec![PolicyAction::Prompt(DetectionId(1), zoom())]);
    assert!(
        SHORT_DWELL <= TimeDelta::seconds(3),
        "the card promises a prompt within 3 s"
    );
}

#[test]
fn an_unknown_app_must_hold_the_microphone_for_fifteen_seconds() {
    let mut harness = Harness::new();
    let unknown = meeting(PHOTO_BOOTH, Dwell::Sustained);

    harness.after(TimeDelta::zero(), Some(&unknown), ON);
    assert!(
        harness
            .after(TimeDelta::seconds(14), Some(&unknown), ON)
            .is_empty()
    );

    assert!(matches!(
        harness
            .after(TimeDelta::seconds(1), Some(&unknown), ON)
            .as_slice(),
        [PolicyAction::Prompt(..)]
    ));
}

#[test]
fn a_brief_hold_or_a_changing_candidate_never_prompts() {
    let mut harness = Harness::new();
    let chrome = meeting(CHROME, Dwell::Short);

    harness.after(TimeDelta::zero(), Some(&zoom()), ON);
    assert!(harness.after(TimeDelta::seconds(2), None, ON).is_empty());
    assert!(
        harness
            .after(TimeDelta::seconds(2), Some(&zoom()), ON)
            .is_empty()
    );
    assert!(
        harness
            .after(TimeDelta::seconds(2), Some(&chrome), ON)
            .is_empty()
    );
    assert!(
        harness
            .after(TimeDelta::seconds(2), Some(&chrome), ON)
            .is_empty()
    );

    assert_eq!(
        harness.policy.next_deadline(),
        Some(start() + TimeDelta::seconds(6) + SHORT_DWELL)
    );
}

#[test]
fn the_prompt_survives_a_flicker_and_ends_after_the_grace() {
    let mut harness = Harness::new();
    let id = harness.prompt_zoom();

    assert!(harness.after(TimeDelta::seconds(1), None, ON).is_empty());
    assert!(
        harness
            .after(TimeDelta::milliseconds(500), Some(&zoom()), ON)
            .is_empty()
    );
    assert!(harness.after(TimeDelta::seconds(1), None, ON).is_empty());

    assert_eq!(
        harness.after(END_GRACE, None, ON),
        vec![PolicyAction::End(id)]
    );
    assert!(harness.policy.prompt().is_none());
}

#[test]
fn an_unanswered_prompt_closes_after_thirty_seconds_and_the_meeting_stays_quiet() {
    let mut harness = Harness::new();
    let id = harness.prompt_zoom();

    assert_eq!(
        harness.after(PROMPT_TIMEOUT, Some(&zoom()), ON),
        vec![PolicyAction::Close(id)]
    );
    assert!(
        harness
            .after(TimeDelta::minutes(5), Some(&zoom()), ON)
            .is_empty()
    );

    // Once that meeting ends, the next one prompts again.
    harness.after(TimeDelta::seconds(1), None, ON);
    harness.after(END_GRACE, None, ON);
    harness.after(TimeDelta::seconds(1), Some(&zoom()), ON);
    assert!(matches!(
        harness.after(SHORT_DWELL, Some(&zoom()), ON).as_slice(),
        [PolicyAction::Prompt(..)]
    ));
}

#[test]
fn dismissing_cools_the_app_down_for_ten_minutes() {
    let mut harness = Harness::new();
    let id = harness.prompt_zoom();

    assert_eq!(harness.policy.answer(id, Answer::Dismiss), Some(zoom()));
    harness.after(TimeDelta::seconds(1), None, ON);
    harness.after(END_GRACE, None, ON);
    harness.after(TimeDelta::seconds(1), Some(&zoom()), ON);
    assert!(
        harness
            .after(TimeDelta::minutes(5), Some(&zoom()), ON)
            .is_empty()
    );

    harness.after(DISMISS_COOLDOWN, Some(&zoom()), ON);
    assert!(matches!(
        harness.after(SHORT_DWELL, Some(&zoom()), ON).as_slice(),
        [PolicyAction::Prompt(..)]
    ));
}

#[test]
fn taking_notes_or_always_ignoring_handles_the_meeting_without_a_cooldown() {
    for answer in [Answer::TakeNotes, Answer::AlwaysIgnore] {
        let mut harness = Harness::new();
        let id = harness.prompt_zoom();

        assert_eq!(harness.policy.answer(id, answer), Some(zoom()));
        assert!(
            harness
                .after(TimeDelta::minutes(1), Some(&zoom()), ON)
                .is_empty(),
            "{answer:?}"
        );
        assert_eq!(
            harness.policy.answer(id, answer),
            None,
            "a second answer finds no prompt"
        );
    }
}

#[test]
fn answering_a_stale_prompt_does_nothing() {
    let mut harness = Harness::new();
    harness.prompt_zoom();

    assert_eq!(
        harness.policy.answer(DetectionId(99), Answer::Dismiss),
        None
    );
    assert!(harness.policy.prompt().is_some());
}

#[test]
fn nothing_prompts_while_recording_and_the_recorded_meeting_stays_quiet_after() {
    let mut harness = Harness::new();

    harness.after(TimeDelta::zero(), Some(&zoom()), RECORDING);
    assert!(
        harness
            .after(TimeDelta::seconds(10), Some(&zoom()), RECORDING)
            .is_empty()
    );
    assert!(
        harness
            .after(TimeDelta::seconds(10), Some(&zoom()), ON)
            .is_empty()
    );
    assert!(
        harness
            .after(TimeDelta::seconds(10), Some(&zoom()), ON)
            .is_empty()
    );
}

#[test]
fn recording_or_switching_detection_off_closes_the_prompt() {
    for gate in [RECORDING, OFF] {
        let mut harness = Harness::new();
        let id = harness.prompt_zoom();

        assert_eq!(
            harness.after(TimeDelta::seconds(1), Some(&zoom()), gate),
            vec![PolicyAction::Close(id)]
        );
        assert!(
            harness
                .after(TimeDelta::seconds(10), Some(&zoom()), gate)
                .is_empty()
        );
    }
}

#[test]
fn deadlines_track_the_dwell_the_timeout_and_the_grace() {
    let mut harness = Harness::new();
    assert_eq!(harness.policy.next_deadline(), None);

    harness.prompt_zoom();
    let shown = start() + SHORT_DWELL;
    assert_eq!(harness.policy.next_deadline(), Some(shown + PROMPT_TIMEOUT));

    harness.after(TimeDelta::seconds(1), None, ON);
    assert_eq!(
        harness.policy.next_deadline(),
        Some(shown + TimeDelta::seconds(1) + END_GRACE)
    );
}
