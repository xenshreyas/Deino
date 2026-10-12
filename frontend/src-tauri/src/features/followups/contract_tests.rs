//! Contract tests: the JSON of the followups commands, through the real
//! plugin, ACL and error mapping, over the seeded fixture library holding
//! one ready draft for the sales discovery call.

use std::sync::Arc;

use deino_core::export::{RecordedClipboard, RecordedOpener};
use deino_core::followups::{
    FollowUpPorts, FollowUpRepo, FollowUpService, FollowUpStatus, Recipients, SqliteFollowUpRepo,
    StoredFollowUp,
};
use deino_domain::{Email, MeetingId};
use deino_testkit::{FakeClock, at, seed_fixtures, test_db};
use serde_json::{Value, json};
use tauri::Manager;
use tauri::test::{MockRuntime, mock_builder};

use crate::features::ipc_tests::{invoke, main_window, mock_app};

const NOW: &str = "2026-09-30T18:00:00Z";
/// The sales discovery fixture.
const DISCOVERY: &str = "meeting-fixture-sales-discovery";

/// The app, and the mail app it opened links in.
struct World {
    app: tauri::App<MockRuntime>,
    opener: Arc<RecordedOpener>,
}

/// An app with the `followups` plugin over the fixtures, holding one ready
/// draft for the discovery call.
fn followups_app() -> World {
    let app = mock_app(mock_builder().plugin(super::init()));
    let database = tauri::async_runtime::block_on(async {
        let database = test_db().await;
        seed_fixtures(&database).await.unwrap();
        database
    });
    let store = Arc::new(SqliteFollowUpRepo::new(database));
    tauri::async_runtime::block_on(store.save(&ready_draft())).unwrap();
    let opener = Arc::new(RecordedOpener::default());
    let service = FollowUpService::new(FollowUpPorts {
        store,
        opener: opener.clone(),
        clipboard: Arc::new(RecordedClipboard::default()),
        clock: Arc::new(FakeClock::new(at(NOW))),
    });
    app.manage(Arc::new(service));
    World { app, opener }
}

fn ready_draft() -> StoredFollowUp {
    StoredFollowUp {
        meeting_id: MeetingId::new(DISCOVERY),
        status: FollowUpStatus::Ready,
        quiet_reason: None,
        subject: "Brightkite pilot".into(),
        body: "Hi Marcus,\n\nProposal by [date].".into(),
        recipients: Recipients {
            to: vec![Email::parse("marcus.webb@brightkite-logistics.com").unwrap()],
            ..Recipients::default()
        },
        generated_at: at(NOW),
        edited_at: None,
        dismissed_at: None,
    }
}

fn call(app: &tauri::App<MockRuntime>, command: &str, args: Value) -> Result<Value, Value> {
    invoke(
        &main_window(app),
        &format!("plugin:followups|{command}"),
        args,
    )
}

fn edit(to: &str) -> Value {
    json!({ "subject": "Pilot", "body": "Hi & bye", "to": [to], "cc": [], "bcc": [] })
}

#[test]
fn suggested_and_for_meeting_return_camel_case_drafts() {
    let world = followups_app();

    let home = call(&world.app, "suggested", json!({})).unwrap();
    let draft = call(&world.app, "for_meeting", json!({ "meeting": DISCOVERY })).unwrap();

    assert_eq!(home[0]["meetingId"], DISCOVERY);
    assert_eq!(
        home[0]["meetingTitle"],
        "Deino × Brightkite Logistics: discovery"
    );
    assert_eq!(draft["subject"], "Brightkite pilot");
    assert_eq!(
        draft["recipients"],
        json!({ "to": ["marcus.webb@brightkite-logistics.com"], "cc": [], "bcc": [] })
    );
    assert_eq!(draft["generatedAt"], NOW);
    assert_eq!(draft["editedAt"], Value::Null);
}

#[test]
fn open_in_mail_saves_the_edit_and_hands_mail_an_encoded_link() {
    let world = followups_app();
    let args = json!({ "meeting": DISCOVERY, "edit": edit("hannah@brightkite-logistics.com") });

    assert_eq!(call(&world.app, "open_in_mail", args).unwrap(), Value::Null);

    assert_eq!(
        world.opener.urls(),
        ["mailto:hannah%40brightkite-logistics.com?subject=Pilot&body=Hi%20%26%20bye"]
    );
    let draft = call(&world.app, "for_meeting", json!({ "meeting": DISCOVERY })).unwrap();
    assert_eq!(draft["editedAt"], NOW);
}

#[test]
fn a_bad_address_is_invalid_and_a_dismissed_draft_is_gone() {
    let world = followups_app();
    let bad = json!({ "meeting": DISCOVERY, "edit": edit("not an address") });

    let error = call(&world.app, "save", bad).unwrap_err();
    assert_eq!(error["code"], "invalid");

    let dismissed = call(&world.app, "dismiss", json!({ "meeting": DISCOVERY }));
    assert_eq!(dismissed.unwrap(), Value::Null);
    let after = call(&world.app, "for_meeting", json!({ "meeting": DISCOVERY }));
    assert_eq!(after.unwrap(), Value::Null);
    assert_eq!(call(&world.app, "suggested", json!({})).unwrap(), json!([]));
}
