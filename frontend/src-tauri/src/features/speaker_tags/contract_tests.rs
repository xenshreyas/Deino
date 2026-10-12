//! Contract tests: the JSON of `plugin:speaker-tags|status` through the
//! real plugin and ACL, over a fake Mac. `open_accessibility_settings` is
//! never invoked here: it would open System Settings on the test machine.

use std::sync::Arc;

use deino_speaker_tags::fake::{FakeMac, MemorySink, ScriptedClock};
use deino_speaker_tags::{SpeakerTags, SpeakerTagsParts};
use serde_json::json;
use tauri::Manager;
use tauri::test::mock_builder;

use crate::features::ipc_tests::{invoke, main_window, mock_app};

/// Speaker tags over a fake Mac, granted or not.
fn tags(mac: &Arc<FakeMac>) -> Arc<SpeakerTags> {
    Arc::new(SpeakerTags::new(SpeakerTagsParts {
        platform: Some(mac.platform()),
        clock: ScriptedClock::new([]),
        sink: Arc::new(MemorySink::default()),
    }))
}

#[test]
fn status_lists_every_app_with_its_state() {
    let app = mock_app(mock_builder().plugin(super::init()));
    let mac = FakeMac::granted();
    mac.set_granted(false);
    app.manage(tags(&mac));
    let window = main_window(&app);

    let rows = invoke(&window, "plugin:speaker-tags|status", json!({})).unwrap();

    assert_eq!(
        rows,
        json!([
            { "app": "zoom", "state": "needsPermission" },
            { "app": "teams", "state": "needsPermission" },
            { "app": "meet", "state": "needsPermission" }
        ])
    );
    mac.set_granted(true);
    let rows = invoke(&window, "plugin:speaker-tags|status", json!({})).unwrap();
    assert_eq!(rows[0], json!({ "app": "zoom", "state": "enabled" }));
}
