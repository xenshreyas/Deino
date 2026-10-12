//! `HotkeyService` over a fake registrar and fake settings: registration at
//! startup, changes, and every kind of conflict.

use std::sync::Arc;

use deino_domain::{ErrorClass, ErrorCode};

use super::{FakeRegistrar, Hotkey, HotkeyAction, HotkeyError, HotkeyService, HotkeyState};
use crate::settings::{
    FakeSettingsStore, RecordedSettingsEvents, Settings, SettingsService, ShortcutSettings,
};

struct Harness {
    registrar: Arc<FakeRegistrar>,
    store: Arc<FakeSettingsStore>,
    service: HotkeyService,
}

async fn harness(shortcuts: ShortcutSettings, registrar: FakeRegistrar) -> Harness {
    let saved = Settings {
        shortcuts,
        ..Settings::default()
    };
    let store = Arc::new(FakeSettingsStore::with(Some(saved)));
    let events = Arc::new(RecordedSettingsEvents::default());
    let settings = Arc::new(SettingsService::load(store.clone(), events).await.unwrap());
    let registrar = Arc::new(registrar);
    let service = HotkeyService::new(registrar.clone(), settings);
    Harness {
        registrar,
        store,
        service,
    }
}

fn state_of(hotkeys: &[Hotkey], action: HotkeyAction) -> HotkeyState {
    hotkeys
        .iter()
        .find(|hotkey| hotkey.action == action)
        .map(|hotkey| hotkey.state.clone())
        .unwrap()
}

fn saved(h: &Harness) -> ShortcutSettings {
    h.store.saved().unwrap().shortcuts
}

#[tokio::test]
async fn startup_registers_the_default_hotkeys() {
    let h = harness(ShortcutSettings::default(), FakeRegistrar::default()).await;
    let hotkeys = h.service.start().await;
    assert!(
        hotkeys
            .iter()
            .all(|hotkey| hotkey.state == HotkeyState::Active)
    );
    let registered = h.registrar.registered();
    assert_eq!(
        registered.get(&HotkeyAction::Record).unwrap(),
        "Ctrl+Alt+Cmd+R"
    );
    assert_eq!(registered.len(), 3);
}

#[tokio::test]
async fn startup_reports_what_it_could_not_register() {
    let shortcuts = ShortcutSettings {
        record: Some("Ctrl+Alt+Cmd+R".to_owned()),
        quick_note: Some("Ctrl+Alt+Cmd+R".to_owned()),
        ask: Some("Cmd+J".to_owned()),
    };
    let h = harness(shortcuts, FakeRegistrar::default()).await;
    let hotkeys = h.service.start().await;
    assert_eq!(
        state_of(&hotkeys, HotkeyAction::Record),
        HotkeyState::Active
    );
    let HotkeyState::Failed { reason } = state_of(&hotkeys, HotkeyAction::QuickNote) else {
        panic!("a duplicate fails");
    };
    assert!(reason.contains("Start or stop recording"), "{reason}");
    assert!(matches!(
        state_of(&hotkeys, HotkeyAction::Ask),
        HotkeyState::Failed { .. }
    ));
    assert_eq!(h.registrar.registered().len(), 1);
}

#[tokio::test]
async fn startup_reports_a_hotkey_another_app_holds() {
    let registrar = FakeRegistrar::default();
    registrar.hold_elsewhere("Ctrl+Alt+Cmd+N");
    let h = harness(ShortcutSettings::default(), registrar).await;
    let hotkeys = h.service.start().await;
    assert_eq!(
        state_of(&hotkeys, HotkeyAction::QuickNote),
        HotkeyState::Failed {
            reason: HotkeyError::Unavailable.to_string()
        }
    );
}

#[tokio::test]
async fn a_new_hotkey_is_registered_then_saved_in_canonical_form() {
    let h = harness(ShortcutSettings::default(), FakeRegistrar::default()).await;
    h.service.start().await;
    let hotkeys = h
        .service
        .set(HotkeyAction::Record, Some("cmd+shift+F9"))
        .await
        .unwrap();
    assert_eq!(
        state_of(&hotkeys, HotkeyAction::Record),
        HotkeyState::Active
    );
    assert_eq!(saved(&h).record.as_deref(), Some("Shift+Cmd+F9"));
    assert_eq!(
        h.registrar.registered().get(&HotkeyAction::Record).unwrap(),
        "Shift+Cmd+F9"
    );
}

#[tokio::test]
async fn turning_a_hotkey_off_releases_and_saves_it() {
    let h = harness(ShortcutSettings::default(), FakeRegistrar::default()).await;
    h.service.start().await;
    let hotkeys = h.service.set(HotkeyAction::Ask, None).await.unwrap();
    assert_eq!(state_of(&hotkeys, HotkeyAction::Ask), HotkeyState::Off);
    assert_eq!(saved(&h).ask, None);
    assert!(!h.registrar.registered().contains_key(&HotkeyAction::Ask));
}

#[tokio::test]
async fn another_deino_hotkeys_combination_is_a_conflict() {
    let h = harness(ShortcutSettings::default(), FakeRegistrar::default()).await;
    h.service.start().await;
    let error = h
        .service
        .set(HotkeyAction::Ask, Some("Ctrl+Alt+Cmd+R"))
        .await
        .unwrap_err();
    assert!(matches!(error, HotkeyError::InUse(HotkeyAction::Record)));
    assert_eq!(error.code(), ErrorCode::Conflict);
    assert_eq!(
        h.registrar.registered().get(&HotkeyAction::Ask).unwrap(),
        "Ctrl+Alt+Cmd+J",
        "the old hotkey keeps working"
    );
}

#[tokio::test]
async fn a_combination_another_app_holds_is_refused_and_the_old_one_restored() {
    let registrar = FakeRegistrar::default();
    registrar.hold_elsewhere("Ctrl+Alt+Space");
    let h = harness(ShortcutSettings::default(), registrar).await;
    h.service.start().await;
    let error = h
        .service
        .set(HotkeyAction::QuickNote, Some("Ctrl+Alt+Space"))
        .await
        .unwrap_err();
    assert!(matches!(error, HotkeyError::Unavailable));
    assert_eq!(
        h.registrar
            .registered()
            .get(&HotkeyAction::QuickNote)
            .unwrap(),
        "Ctrl+Alt+Cmd+N"
    );
    assert!(h.store.saved().unwrap().shortcuts.quick_note.is_some());
    let listed = h.service.list().await;
    assert_eq!(
        state_of(&listed, HotkeyAction::QuickNote),
        HotkeyState::Active
    );
}

#[tokio::test]
async fn an_invalid_combination_is_refused_before_anything_changes() {
    let h = harness(ShortcutSettings::default(), FakeRegistrar::default()).await;
    h.service.start().await;
    let error = h
        .service
        .set(HotkeyAction::Record, Some("Cmd+R"))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::Invalid);
    assert_eq!(h.registrar.registered().len(), 3);
}

#[tokio::test]
async fn a_failed_save_restores_the_old_hotkey() {
    let h = harness(ShortcutSettings::default(), FakeRegistrar::default()).await;
    h.service.start().await;
    h.store.fail_saves();
    let error = h
        .service
        .set(HotkeyAction::Record, Some("Ctrl+Alt+F2"))
        .await
        .unwrap_err();
    assert!(matches!(error, HotkeyError::Settings(_)));
    assert_eq!(
        h.registrar.registered().get(&HotkeyAction::Record).unwrap(),
        "Ctrl+Alt+Cmd+R"
    );
}

#[test]
fn each_action_runs_its_intent() {
    use crate::links::{Intent, Navigation};
    assert_eq!(HotkeyAction::Record.intent(), Intent::ToggleRecording);
    assert_eq!(
        HotkeyAction::QuickNote.intent(),
        Intent::Show(Navigation::NewNote { event_id: None })
    );
    assert_eq!(HotkeyAction::Ask.intent(), Intent::Show(Navigation::Ask));
}
