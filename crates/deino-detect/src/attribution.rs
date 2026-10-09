//! Which app a capturing process belongs to.
//!
//! The rules are pure, over the [`AppDirectory`] port, so they are tested
//! without a Mac:
//!
//! - a process maps to the **outermost** `.app` bundle around its executable,
//!   so `Google Chrome Helper.app` inside `Google Chrome.app` is Chrome
//!   (anarlog `crates/detect/src/list/macos.rs`, MIT);
//! - Apple's call daemons (`avconferenced`, `callservicesd`) become FaceTime
//!   or an iPhone call, by which of the two is frontmost or running (anarlog);
//! - Safari's microphone is captured by the shared WebKit GPU process, which
//!   is credited to Safari while Safari runs;
//! - a process outside any bundle is named by its executable
//!   (`process.ffmpeg`), so it can still be ignored.

use std::path::Path;

use deino_domain::BundleId;

use crate::classify::browser_of;
use crate::model::{AppIdentity, AudioProcess, Pid};
use crate::ports::{AppDirectory, RunningApp};

/// Apple's daemons that capture the microphone for FaceTime and phone calls.
const CALL_DAEMONS: &[&str] = &[
    "com.apple.avconferenced",
    "com.apple.TelephonyUtilities",
    "com.apple.TelephonyUtilities.callservicesd",
    "process.avconferenced",
    "process.callservicesd",
];

/// The WebKit process that captures audio for Safari's pages.
const WEBKIT_GPU: &str = "com.apple.WebKit.GPU";

/// The apps a call daemon is relabelled as.
const FACETIME: &str = "com.apple.FaceTime";
const PHONE: &str = "com.apple.mobilephone";
const SAFARI: &str = "com.apple.Safari";

/// The name an iPhone call relayed to the Mac is shown with.
const PHONE_CALL_NAME: &str = "iPhone Call";

/// Prefix of the identifier given to a process outside any bundle.
const PROCESS_PREFIX: &str = "process.";

/// The app `process` belongs to, or `None` when it has gone or cannot be named.
pub fn attribute(process: &AudioProcess, directory: &dyn AppDirectory) -> Option<AppIdentity> {
    let app = directory
        .app_for_pid(process.pid)
        .or_else(|| app_by_bundle(process.bundle_id.as_ref()?, directory))?;
    let reported = process.bundle_id.as_ref();
    if is_one_of(&app.bundle_id, CALL_DAEMONS)
        || reported.is_some_and(|id| is_one_of(id, CALL_DAEMONS))
    {
        return Some(call_app(directory).unwrap_or(app));
    }
    if reported.is_some_and(|id| id.as_str() == WEBKIT_GPU) || app.bundle_id.as_str() == WEBKIT_GPU
    {
        return Some(running(SAFARI, directory).map_or(app, |safari| safari.app));
    }
    Some(app)
}

/// The outermost `.app` bundle on `path`, if the path lies inside one.
pub fn outermost_app_bundle(path: &Path) -> Option<&Path> {
    path.ancestors()
        .filter(|ancestor| {
            ancestor
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("app"))
        })
        .last()
}

/// The identity of a process outside any bundle, from its executable name.
pub fn process_identity(executable_name: &str, pid: Pid) -> Option<AppIdentity> {
    let name: String = executable_name
        .chars()
        .map(|c| {
            if c.is_whitespace() || c.is_control() {
                '-'
            } else {
                c
            }
        })
        .collect();
    let id = if name.is_empty() {
        format!("{PROCESS_PREFIX}{}", pid.0)
    } else {
        format!("{PROCESS_PREFIX}{name}")
    };
    let bundle_id = BundleId::try_from(id).ok()?;
    let name = if executable_name.is_empty() {
        bundle_id.to_string()
    } else {
        executable_name.to_owned()
    };
    Some(AppIdentity { bundle_id, name })
}

/// The app for a process that has already exited, from the bundle Core
/// Audio reported: a browser helper maps to its browser.
fn app_by_bundle(bundle_id: &BundleId, directory: &dyn AppDirectory) -> Option<AppIdentity> {
    let owner = browser_of(bundle_id).unwrap_or_else(|| bundle_id.clone());
    let named = directory.running_app(&owner).map(|running| running.app);
    named.or_else(|| {
        Some(AppIdentity {
            name: owner.to_string(),
            bundle_id: owner,
        })
    })
}

/// FaceTime or an iPhone call: the frontmost one, else the only one running
/// (anarlog `select_apple_call_app_name`).
fn call_app(directory: &dyn AppDirectory) -> Option<AppIdentity> {
    let facetime = running(FACETIME, directory);
    let phone = running(PHONE, directory).map(|mut phone| {
        PHONE_CALL_NAME.clone_into(&mut phone.app.name);
        phone
    });
    match (facetime, phone) {
        (Some(facetime), _) if facetime.active => Some(facetime.app),
        (_, Some(phone)) if phone.active => Some(phone.app),
        (Some(facetime), None) => Some(facetime.app),
        (None, Some(phone)) => Some(phone.app),
        _ => None,
    }
}

fn running(bundle_id: &str, directory: &dyn AppDirectory) -> Option<RunningApp> {
    directory.running_app(&BundleId::try_from(bundle_id).ok()?)
}

fn is_one_of(bundle_id: &BundleId, table: &[&str]) -> bool {
    table.contains(&bundle_id.as_str())
}

#[cfg(test)]
#[path = "attribution_tests.rs"]
mod tests;
