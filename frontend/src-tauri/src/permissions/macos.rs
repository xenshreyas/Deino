//! macOS permissions: `AVFoundation` for the microphone, the process-tap
//! probe for system audio, `AXIsProcessTrusted` for Accessibility,
//! `CGPreflightScreenCaptureAccess` for Screen Recording (CAP-09), and the
//! System Settings privacy pages (§3C).

use cidre::av::{self, capture::AuthorizationStatus};
use cidre::blocks::Completion;
use deino_core::permissions::{PermissionKind, PermissionState};
use objc2_app_kit::NSWorkspace;
use objc2_foundation::{NSString, NSURL};

/// System Settings → Privacy & Security, before the page's anchor.
const PRIVACY: &str = "x-apple.systempreferences:com.apple.preference.security?";
/// System Settings → Notifications.
const NOTIFICATIONS_PAGE: &str =
    "x-apple.systempreferences:com.apple.Notifications-Settings.extension";

/// The microphone's authorization, read without prompting.
pub(super) fn microphone() -> PermissionState {
    av::CaptureDevice::authorization_status_for_media_type(av::MediaType::audio())
        .map_or(PermissionState::Unknown, from_av)
}

/// Asks for the microphone; macOS prompts only while it is undetermined.
pub(super) async fn request_microphone() -> PermissionState {
    let Some(answer) = ask_microphone() else {
        return PermissionState::Unknown;
    };
    if answer.await {
        PermissionState::Granted
    } else {
        PermissionState::Denied
    }
}

/// Sends the request and returns its answer. Kept synchronous so no
/// Objective-C reference (which is not `Send`) lives across an await.
fn ask_microphone() -> Option<Completion<bool>> {
    let (answer, mut block) = cidre::blocks::comp1();
    av::CaptureDevice::request_access_for_media_type_ch(av::MediaType::audio(), &mut block).ok()?;
    Some(answer)
}

/// macOS has no public way to read the audio capture permission.
pub(super) const fn system_audio() -> PermissionState {
    PermissionState::Unknown
}

/// Starts and stops the recorder's process tap, which makes macOS ask for
/// audio capture the first time (the legacy probe, kept in one place).
pub(super) async fn request_system_audio() -> PermissionState {
    let probe = tauri::async_runtime::spawn_blocking(
        crate::audio::permissions::trigger_system_audio_permission,
    );
    match probe.await {
        Ok(Ok(true)) => PermissionState::Granted,
        Ok(Ok(false)) => PermissionState::Denied,
        Ok(Err(_)) | Err(_) => PermissionState::Unknown,
    }
}

/// Whether Deino is trusted for Accessibility. macOS cannot tell "never
/// asked" from "refused", so an untrusted app is reported undetermined.
pub(super) fn accessibility() -> PermissionState {
    trust(cidre::ax::is_process_trusted())
}

/// Shows the macOS Accessibility prompt, which leads to System Settings.
pub(super) fn request_accessibility() -> PermissionState {
    trust(cidre::ax::is_process_trusted_with_prompt(true))
}

/// Whether Screen Recording is granted (`CGPreflightScreenCaptureAccess`,
/// which never prompts). macOS cannot tell "never asked" from "refused".
pub(super) fn screen_recording() -> PermissionState {
    trust(deino_screen::platform().is_some_and(|platform| platform.permission.granted()))
}

/// Asks for Screen Recording from the user's click (CAP-09): the macOS
/// prompt the first time, which leads to System Settings.
pub(super) fn request_screen_recording() -> PermissionState {
    trust(deino_screen::request_screen_recording())
}

/// Opens the System Settings page where `kind` is granted.
pub(super) fn open_settings(kind: PermissionKind) -> bool {
    let page = match kind {
        PermissionKind::Microphone => format!("{PRIVACY}Privacy_Microphone"),
        PermissionKind::SystemAudio => format!("{PRIVACY}Privacy_ScreenCapture"),
        PermissionKind::Accessibility => format!("{PRIVACY}Privacy_Accessibility"),
        PermissionKind::ScreenRecording => format!("{PRIVACY}Privacy_ScreenCapture"),
        PermissionKind::Notifications => NOTIFICATIONS_PAGE.to_owned(),
    };
    let Some(url) = NSURL::URLWithString(&NSString::from_str(&page)) else {
        return false;
    };
    NSWorkspace::sharedWorkspace().openURL(&url)
}

const fn trust(trusted: bool) -> PermissionState {
    if trusted {
        PermissionState::Granted
    } else {
        PermissionState::NotDetermined
    }
}

const fn from_av(status: AuthorizationStatus) -> PermissionState {
    match status {
        AuthorizationStatus::Authorized => PermissionState::Granted,
        AuthorizationStatus::Denied | AuthorizationStatus::Restricted => PermissionState::Denied,
        AuthorizationStatus::NotDetermined => PermissionState::NotDetermined,
    }
}
