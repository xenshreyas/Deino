//! The permission types the onboarding UI shows.

use serde::{Deserialize, Serialize};

/// A permission Deino asks macOS for.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, specta::Type,
)]
#[serde(rename_all = "snake_case")]
pub enum PermissionKind {
    /// The microphone: the user's side of every meeting.
    Microphone,
    /// System audio capture through the Core Audio process tap: the other
    /// side of every meeting.
    SystemAudio,
    /// Notifications: meeting reminders and detected meetings.
    Notifications,
    /// Accessibility: speaker names from the meeting app's window (CAP-05)
    /// and browser meeting URLs (CAP-06).
    Accessibility,
}

impl PermissionKind {
    /// Every kind, in the order onboarding lists them.
    pub const ALL: [Self; 4] = [
        Self::Microphone,
        Self::SystemAudio,
        Self::Notifications,
        Self::Accessibility,
    ];

    /// Whether recording works without it (only Accessibility is optional).
    pub const fn optional(self) -> bool {
        matches!(self, Self::Accessibility)
    }
}

/// Where a permission stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum PermissionState {
    /// Granted.
    Granted,
    /// Refused or restricted; only System Settings can change it.
    Denied,
    /// Not asked yet; asking shows the macOS prompt.
    NotDetermined,
    /// macOS cannot tell without asking, and Deino has not asked yet.
    Unknown,
    /// Not needed on this platform.
    Unsupported,
}

impl PermissionState {
    /// Whether the permission still needs the user's attention.
    pub const fn needs_action(self) -> bool {
        !matches!(self, Self::Granted | Self::Unsupported)
    }
}

/// One permission as the UI shows it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PermissionStatus {
    /// Which permission.
    pub kind: PermissionKind,
    /// Where it stands.
    pub state: PermissionState,
    /// Whether recording works without it.
    pub optional: bool,
}

impl PermissionStatus {
    /// The status of `kind` in `state`.
    pub const fn new(kind: PermissionKind, state: PermissionState) -> Self {
        Self {
            kind,
            state,
            optional: kind.optional(),
        }
    }
}
