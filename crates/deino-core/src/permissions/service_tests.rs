//! [`PermissionService`] over the fake gate.

use std::sync::Arc;

use deino_domain::{AppError, ErrorCode};

use super::*;

fn service(gate: FakePermissionGate) -> (PermissionService, Arc<FakePermissionGate>) {
    let gate = Arc::new(gate);
    (PermissionService::new(gate.clone()), gate)
}

#[test]
fn every_permission_is_listed_in_order_with_accessibility_optional() {
    let (service, gate) = service(FakePermissionGate::default());
    gate.set(PermissionKind::Microphone, PermissionState::Granted);

    let statuses = service.statuses();

    let kinds: Vec<_> = statuses.iter().map(|status| status.kind).collect();
    assert_eq!(kinds, PermissionKind::ALL);
    assert_eq!(statuses[0].state, PermissionState::Granted);
    assert_eq!(statuses[1].state, PermissionState::NotDetermined);
    let optional: Vec<_> = statuses
        .iter()
        .filter(|s| s.optional)
        .map(|s| s.kind)
        .collect();
    assert_eq!(optional, [PermissionKind::Accessibility]);
}

#[tokio::test]
async fn a_request_reports_the_new_state() {
    let (service, gate) = service(FakePermissionGate::default());
    gate.refuse(PermissionKind::Microphone, PermissionState::Denied);

    let status = service.request(PermissionKind::Microphone).await;

    assert_eq!(
        status,
        PermissionStatus::new(PermissionKind::Microphone, PermissionState::Denied)
    );
}

#[tokio::test]
async fn an_unreadable_permission_remembers_the_last_answer() {
    let (service, gate) = service(FakePermissionGate::default());
    gate.set(PermissionKind::SystemAudio, PermissionState::Unknown);
    assert_eq!(service.statuses()[1].state, PermissionState::Unknown);

    let status = service.request(PermissionKind::SystemAudio).await;

    assert_eq!(status.state, PermissionState::Granted);
    assert_eq!(service.statuses()[1].state, PermissionState::Granted);
}

#[tokio::test]
async fn a_readable_permission_always_reports_the_live_state() {
    let (service, gate) = service(FakePermissionGate::default());
    service.request(PermissionKind::Accessibility).await;

    gate.set(
        PermissionKind::Accessibility,
        PermissionState::NotDetermined,
    );

    assert_eq!(service.statuses()[3].state, PermissionState::NotDetermined);
}

#[test]
fn opening_settings_reaches_the_gate_and_reports_failure_as_unavailable() {
    let (service, gate) = service(FakePermissionGate::without_settings());

    let error = service
        .open_settings(PermissionKind::Notifications)
        .unwrap_err();

    assert_eq!(gate.opened(), [PermissionKind::Notifications]);
    assert_eq!(AppError::from(error).code, ErrorCode::Unavailable);
}

#[test]
fn only_granted_or_unneeded_permissions_need_no_action() {
    let needing: Vec<_> = [
        PermissionState::Granted,
        PermissionState::Denied,
        PermissionState::NotDetermined,
        PermissionState::Unknown,
        PermissionState::Unsupported,
    ]
    .into_iter()
    .filter(|state| state.needs_action())
    .collect();

    assert_eq!(
        needing,
        [
            PermissionState::Denied,
            PermissionState::NotDetermined,
            PermissionState::Unknown
        ]
    );
}
