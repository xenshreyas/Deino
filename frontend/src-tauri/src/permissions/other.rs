//! Windows and Linux: recording needs none of the macOS permissions, and
//! there is no System Settings page to open (D1: compile-only).

use deino_core::permissions::{PermissionKind, PermissionState};

pub(super) const fn microphone() -> PermissionState {
    PermissionState::Unsupported
}

pub(super) async fn request_microphone() -> PermissionState {
    PermissionState::Unsupported
}

pub(super) const fn system_audio() -> PermissionState {
    PermissionState::Unsupported
}

pub(super) async fn request_system_audio() -> PermissionState {
    PermissionState::Unsupported
}

pub(super) const fn accessibility() -> PermissionState {
    PermissionState::Unsupported
}

pub(super) const fn request_accessibility() -> PermissionState {
    PermissionState::Unsupported
}

pub(super) const fn open_settings(_kind: PermissionKind) -> bool {
    false
}
