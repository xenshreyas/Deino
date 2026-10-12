//! CAP-05's fixture dumps (`deino-speaker-tags/fixtures/<app>/<case>.json`),
//! which CAP-13 extends with the meeting chat (`chat-*.json`).

use std::path::PathBuf;
use std::sync::Arc;

use deino_consent::{ChatView, chat_adapter_for};
use deino_domain::CallApp;
use deino_speaker_tags::fake::FakeMac;
use deino_speaker_tags::{AxNode, Pid, SpeakerAdapter};

/// The windows of one dump.
pub fn windows(app: CallApp, case: &str) -> Vec<AxNode> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../deino-speaker-tags/fixtures")
        .join(app.as_str())
        .join(format!("{case}.json"));
    let json = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    serde_json::from_str(&json).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

/// What each call window of a dump shows of the chat, as the session sees
/// it (windows the speaker adapter does not take for a call are skipped).
pub fn views(app: CallApp, case: &str) -> Vec<(AxNode, ChatView)> {
    let adapter = chat_adapter_for(app);
    windows(app, case)
        .into_iter()
        .filter(|window| adapter.speaker().is_call_window(&window.summary()))
        .map(|window| {
            let view = adapter.view(&window);
            (window, view)
        })
        .collect()
}

/// The process id the fake Mac runs every app under.
pub const PID: Pid = Pid(4242);

/// A fake Mac with Accessibility granted, running `app` and showing `case`.
pub fn mac(app: CallApp, case: &str) -> Arc<FakeMac> {
    let mac = FakeMac::granted();
    show(&mac, app, case);
    mac
}

/// Runs `app` on `mac` and shows `case` from now on.
pub fn show(mac: &FakeMac, app: CallApp, case: &str) {
    let speaker: &dyn SpeakerAdapter = chat_adapter_for(app).speaker();
    if let Some(bundle) = speaker.bundle_ids().first() {
        mac.run(bundle, PID);
    }
    mac.show(PID, Ok(windows(app, case)));
}
