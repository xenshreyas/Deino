//! The start observer (ORG-03): it hears every recording that starts, with
//! whether the caller named it, and the title it gives the meeting is the
//! one the recorder shows.
#![allow(
    clippy::unwrap_used,
    reason = "an integration-test crate: helpers fail the test by panicking"
)]

#[path = "support/rig.rs"]
mod rig;

use std::sync::{Arc, Mutex, PoisonError};

use async_trait::async_trait;
use deino_audio::TestSignal;
use deino_audio::capture::OutputRoute;
use deino_domain::MeetingId;
use deino_recording::{RecordingService, StartObserver};
use rig::{FastClock, FixtureCapture, rig};

/// Names untitled meetings "Design review", like a linked calendar event.
#[derive(Default)]
struct Linker {
    heard: Mutex<Vec<(MeetingId, bool)>>,
}

#[async_trait]
impl StartObserver for Linker {
    async fn started(&self, meeting: &MeetingId, titled: bool) -> Option<String> {
        let mut heard = self.heard.lock().unwrap_or_else(PoisonError::into_inner);
        heard.push((meeting.clone(), titled));
        (!titled).then(|| "Design review".to_owned())
    }
}

fn voices() -> FixtureCapture {
    FixtureCapture {
        clock: Arc::new(FastClock::new(4)),
        microphone: TestSignal::microphone(),
        system: TestSignal::system(),
        route: OutputRoute::Headphones,
    }
}

/// Starts one recording titled `title` with a fresh [`Linker`]; the
/// meeting, the title the recorder shows, and what the linker heard.
async fn start(title: Option<&str>) -> (MeetingId, Option<String>, Vec<(MeetingId, bool)>) {
    let rig = rig(voices()).await;
    let linker = Arc::new(Linker::default());
    let service = RecordingService::new(rig.deps.clone()).observing_starts(linker.clone());

    let meeting = service.start(title.map(str::to_owned)).await.unwrap();

    let heard = linker.heard.lock().unwrap().clone();
    (meeting, service.state().title, heard)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_untitled_start_shows_the_title_the_observer_gave_it() {
    let (meeting, shown, heard) = start(None).await;

    assert_eq!(shown.as_deref(), Some("Design review"));
    assert_eq!(heard, [(meeting, false)]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_titled_start_keeps_its_title() {
    let (meeting, shown, heard) = start(Some("Interview prep")).await;

    assert_eq!(shown.as_deref(), Some("Interview prep"));
    assert_eq!(heard, [(meeting, true)]);
}
