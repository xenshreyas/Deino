//! The notice end to end on fixture dumps and fakes: when it posts, the
//! typing guard at every step, the empty-composer rule, once per meeting,
//! and every way it degrades. Nothing here touches a real app.

#![allow(
    clippy::unwrap_used,
    clippy::panic,
    reason = "an integration-test crate: helpers fail the test by panicking"
)]

#[path = "support/dumps.rs"]
mod dumps;

use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;

use deino_consent::fake::{
    Act, FakeChat, FakeRecorder, FixedConfig, Moment, ScriptedKeys, platform,
};
use deino_consent::{
    ChatConfig, ChatView, MAX_FAILURES, Monitor, NodePath, NoticeParts, Progress, RecorderNow,
};
use deino_domain::{CallApp, MeetingId};
use deino_speaker_tags::fake::FakeMac;

const MESSAGE: &str = "Deino is transcribing this call for my notes.";

/// Typing a moment ago, and quiet for a while.
const TYPING: Option<Duration> = Some(Duration::from_millis(400));
const QUIET: Option<Duration> = Some(Duration::from_secs(30));

/// One fake call: a Mac showing a dump, the chat, the keyboard, the recorder.
struct Call {
    mac: Arc<FakeMac>,
    chat: Arc<FakeChat>,
    recorder: Arc<FakeRecorder>,
    config: Arc<FixedConfig>,
    monitor: Monitor,
}

fn on(apps: &[CallApp]) -> ChatConfig {
    ChatConfig {
        message: MESSAGE.into(),
        apps: apps.to_vec(),
    }
}

fn call(app: CallApp, case: &str, keys: Arc<ScriptedKeys>) -> Call {
    let mac = dumps::mac(app, case);
    let chat = FakeChat::new();
    let recorder = FakeRecorder::recording("meeting", true);
    let config = FixedConfig::new(on(&[app]));
    let monitor = Monitor::new(NoticeParts {
        platform: Some(platform(&mac, &chat, keys)),
        recorder: recorder.clone(),
        config: config.clone(),
    });
    Call {
        mac,
        chat,
        recorder,
        config,
        monitor,
    }
}

impl Call {
    fn ticks(&mut self, count: usize) {
        for _ in 0..count {
            self.monitor.tick();
        }
    }

    fn progress(&self) -> Option<Progress> {
        self.monitor.progress(&MeetingId::new("meeting"))
    }
}

/// The open composer's and send button's paths in a dump.
fn composer(app: CallApp, case: &str) -> (NodePath, NodePath) {
    dumps::views(app, case)
        .into_iter()
        .find_map(|(_, view)| match view {
            ChatView::Open(composer) => Some((composer.field, composer.send?)),
            _ => None,
        })
        .unwrap()
}

#[test]
fn it_posts_once_into_the_empty_composer_of_each_app() {
    for app in CallApp::ALL {
        let mut call = call(*app, "chat-open-empty", ScriptedKeys::quiet());
        let (field, send) = composer(*app, "chat-open-empty");

        call.ticks(5);

        assert_eq!(
            call.chat.acts(),
            [
                Act::Focus(field.clone()),
                Act::Set(field, MESSAGE.into()),
                Act::Press(send)
            ],
            "{app}"
        );
        assert_eq!(call.progress(), Some(Progress::Sent), "{app}");
    }
}

#[test]
fn it_never_posts_twice_in_a_meeting_even_after_a_pause_or_a_resume() {
    let mut call = call(CallApp::Zoom, "chat-open-empty", ScriptedKeys::quiet());
    call.ticks(2);
    call.recorder
        .set(RecorderNow::Paused(MeetingId::new("meeting")));
    call.ticks(2);
    call.recorder.set(RecorderNow::Idle);
    call.ticks(1);
    call.recorder
        .set(RecorderNow::Recording(MeetingId::new("meeting")));
    call.ticks(5);

    assert_eq!(call.chat.presses(), 1);

    // A new meeting gets its own notice.
    call.recorder
        .set(RecorderNow::Recording(MeetingId::new("next")));
    call.ticks(1);
    assert_eq!(call.chat.presses(), 2);
}

#[test]
fn it_never_touches_the_chat_while_the_user_is_typing() {
    let mut call = call(
        CallApp::Meet,
        "chat-open-empty",
        ScriptedKeys::new([TYPING]),
    );

    call.ticks(20);

    assert!(call.chat.acts().is_empty());
    assert_eq!(call.progress(), Some(Progress::Waiting));
}

#[test]
fn unknown_key_activity_counts_as_typing() {
    let mut call = call(CallApp::Zoom, "chat-open-empty", ScriptedKeys::new([None]));

    call.ticks(5);

    assert!(call.chat.acts().is_empty());
}

#[test]
fn typing_after_the_focus_stops_before_anything_is_written() {
    // Checks: gate, attempt start, after focus (typing), then quiet again.
    let keys = ScriptedKeys::new([QUIET, QUIET, TYPING, QUIET]);
    let mut call = call(CallApp::Teams, "chat-open-empty", keys);
    let (field, _) = composer(CallApp::Teams, "chat-open-empty");

    call.ticks(1);

    assert_eq!(call.chat.acts(), [Act::Focus(field)]);
    assert_eq!(call.progress(), Some(Progress::Waiting));
    call.ticks(1);
    assert_eq!(call.chat.presses(), 1);
}

#[test]
fn typing_after_the_write_takes_the_notice_back_out_and_waits() {
    // Checks: gate, start, after focus, before send (typing).
    let keys = ScriptedKeys::new([QUIET, QUIET, QUIET, TYPING, TYPING]);
    let mut call = call(CallApp::Zoom, "chat-open-empty", keys);
    let (field, _) = composer(CallApp::Zoom, "chat-open-empty");

    call.ticks(3);

    assert_eq!(
        call.chat.acts(),
        [
            Act::Focus(field.clone()),
            Act::Set(field.clone(), MESSAGE.into()),
            Act::Set(field.clone(), String::new()),
        ]
    );
    assert_eq!(call.chat.text(&field), "");
    assert_eq!(call.progress(), Some(Progress::Waiting));
}

#[test]
fn text_typed_into_the_composer_meanwhile_is_left_alone_and_never_sent() {
    let mut call = call(CallApp::Meet, "chat-open-empty", ScriptedKeys::quiet());
    let (field, _) = composer(CallApp::Meet, "chat-open-empty");
    call.chat.user_types_at(Moment::AfterWrite, " hi");

    call.ticks(5);

    assert_eq!(call.chat.presses(), 0);
    assert_eq!(call.chat.text(&field), format!("{MESSAGE} hi"));
    assert_eq!(call.progress(), Some(Progress::GaveUp));
}

#[test]
fn a_composer_found_empty_but_holding_text_at_the_write_is_not_touched() {
    let mut call = call(CallApp::Zoom, "chat-open-empty", ScriptedKeys::quiet());
    let (field, _) = composer(CallApp::Zoom, "chat-open-empty");
    call.chat.type_text(&field, "brb");

    call.ticks(3);

    assert!(call.chat.acts().is_empty());
    assert_eq!(call.chat.text(&field), "brb");
}

#[test]
fn a_draft_in_the_composer_waits() {
    for app in CallApp::ALL {
        let mut call = call(*app, "chat-open-drafting", ScriptedKeys::quiet());

        call.ticks(5);

        assert!(call.chat.acts().is_empty(), "{app}");
    }
}

#[test]
fn it_waits_for_speech_and_for_transcription() {
    let mut call = call(CallApp::Zoom, "chat-open-empty", ScriptedKeys::quiet());
    call.recorder.hear(false);
    call.ticks(3);
    assert!(call.chat.acts().is_empty());

    call.recorder
        .set(RecorderNow::Paused(MeetingId::new("meeting")));
    call.recorder.hear(true);
    call.ticks(3);
    assert!(call.chat.acts().is_empty());

    call.recorder
        .set(RecorderNow::Recording(MeetingId::new("meeting")));
    call.ticks(1);
    assert_eq!(call.chat.presses(), 1);
}

#[test]
fn it_is_off_until_an_app_is_switched_on_with_a_message() {
    let mut call = call(CallApp::Zoom, "chat-open-empty", ScriptedKeys::quiet());
    call.config.set(ChatConfig::default());
    call.ticks(3);
    call.config.set(on(&[CallApp::Teams]));
    call.ticks(3);
    call.config.set(ChatConfig {
        message: "  ".into(),
        apps: vec![CallApp::Zoom],
    });
    call.ticks(3);

    assert!(call.chat.acts().is_empty());
}

#[test]
fn a_closed_chat_is_opened_first_then_posted_to() {
    let mut call = call(CallApp::Meet, "call-alex-speaking", ScriptedKeys::quiet());

    call.ticks(1);
    let acts = call.chat.acts();
    assert!(matches!(acts.as_slice(), [Act::Press(_)]), "{acts:?}");

    dumps::show(&call.mac, CallApp::Meet, "chat-open-empty");
    call.ticks(1);
    assert_eq!(call.chat.presses(), 2);
    assert_eq!(call.progress(), Some(Progress::Sent));
}

#[test]
fn without_permission_nothing_is_read_or_written() {
    let mut call = call(CallApp::Zoom, "chat-open-empty", ScriptedKeys::quiet());
    call.mac.set_granted(false);

    call.ticks(5);

    assert!(call.chat.acts().is_empty());
    assert!(call.mac.prepared().is_empty());
}

#[test]
fn an_app_that_ignores_the_write_gives_up_without_sending() {
    let mut call = call(CallApp::Teams, "chat-open-empty", ScriptedKeys::quiet());
    call.chat.ignores_writes.store(true, Ordering::SeqCst);

    call.ticks(5);

    assert_eq!(call.chat.presses(), 0);
    assert_eq!(call.progress(), Some(Progress::GaveUp));
}

#[test]
fn a_send_that_keeps_the_text_takes_it_back_and_gives_up() {
    let mut call = call(CallApp::Zoom, "chat-open-empty", ScriptedKeys::quiet());
    let (field, _) = composer(CallApp::Zoom, "chat-open-empty");
    call.chat.keeps_text_on_press.store(true, Ordering::SeqCst);

    call.ticks(5);

    assert_eq!(call.chat.presses(), 1);
    assert_eq!(call.chat.text(&field), "");
    assert_eq!(call.progress(), Some(Progress::GaveUp));
}

#[test]
fn a_window_that_keeps_changing_is_given_up_after_a_few_tries() {
    let mut call = call(CallApp::Zoom, "chat-open-empty", ScriptedKeys::quiet());
    call.chat.moved.store(true, Ordering::SeqCst);

    call.ticks(usize::from(MAX_FAILURES) - 1);
    assert_eq!(call.progress(), Some(Progress::Waiting));
    call.ticks(5);

    assert_eq!(call.progress(), Some(Progress::GaveUp));
    assert!(call.chat.acts().is_empty());
}

#[test]
fn no_platform_posts_nothing() {
    let recorder = FakeRecorder::recording("meeting", true);
    let mut monitor = Monitor::new(NoticeParts {
        platform: None,
        recorder,
        config: FixedConfig::new(on(&[CallApp::Zoom])),
    });

    monitor.tick();

    assert_eq!(
        monitor.progress(&MeetingId::new("meeting")),
        Some(Progress::Waiting)
    );
}
