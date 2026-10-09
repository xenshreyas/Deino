//! [`Workspace`]: running apps through `NSWorkspace` and
//! `NSRunningApplication`, and the outermost `.app` a process belongs to.
//!
//! A helper's own bundle (`Google Chrome Helper (Renderer).app`) sits inside
//! its parent's, so the outermost bundle on its path is the app the user
//! knows. A process outside any registered app (a daemon, a command-line
//! tool) is found through its executable path instead.

use std::path::{Path, PathBuf};

use deino_domain::BundleId;
use objc2_app_kit::{NSRunningApplication, NSWorkspace};
use objc2_foundation::{NSBundle, NSString};
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};

use crate::attribution::{outermost_app_bundle, process_identity};
use crate::model::{AppIdentity, Pid};
use crate::ports::{AppDirectory, RunningApp};

/// The running apps of this Mac.
pub(super) struct Workspace;

impl AppDirectory for Workspace {
    fn app_for_pid(&self, pid: Pid) -> Option<AppIdentity> {
        match NSRunningApplication::runningApplicationWithProcessIdentifier(pid.0) {
            Some(app) => identity_of(&app),
            None => unregistered(pid),
        }
    }

    fn running_app(&self, bundle_id: &BundleId) -> Option<RunningApp> {
        let id = NSString::from_str(bundle_id.as_str());
        let apps = NSRunningApplication::runningApplicationsWithBundleIdentifier(&id);
        apps.iter().find_map(|app| running(&app))
    }

    fn frontmost(&self) -> Option<RunningApp> {
        let front = NSWorkspace::sharedWorkspace().frontmostApplication()?;
        running(&front)
    }
}

fn running(app: &NSRunningApplication) -> Option<RunningApp> {
    Some(RunningApp {
        app: identity_of(app)?,
        pid: Pid(app.processIdentifier()),
        active: app.isActive(),
    })
}

/// The app the user knows `app` as: its outermost enclosing bundle.
fn identity_of(app: &NSRunningApplication) -> Option<AppIdentity> {
    let own = app
        .bundleURL()
        .or_else(|| app.executableURL())
        .and_then(|url| url.path())
        .map(|path| PathBuf::from(path.to_string()));
    let outer = own.as_deref().and_then(outermost_app_bundle);
    if let Some(outer) = outer.filter(|outer| Some(*outer) != own.as_deref()) {
        return bundle_identity(outer);
    }
    let bundle_id = BundleId::try_from(app.bundleIdentifier()?.to_string()).ok()?;
    let name = app
        .localizedName()
        .map_or_else(|| bundle_id.to_string(), |name| name.to_string());
    Some(AppIdentity { bundle_id, name })
}

/// A process Launch Services does not list, from its executable path.
fn unregistered(pid: Pid) -> Option<AppIdentity> {
    let id = sysinfo::Pid::from_u32(u32::try_from(pid.0).ok()?);
    let mut system = System::new();
    system.refresh_processes_specifics(
        ProcessesToUpdate::Some(&[id]),
        true,
        ProcessRefreshKind::new().with_exe(sysinfo::UpdateKind::Always),
    );
    let process = system.process(id)?;
    if let Some(outer) = process.exe().and_then(outermost_app_bundle) {
        return bundle_identity(outer);
    }
    process_identity(&process.name().to_string_lossy(), pid)
}

/// The identifier and name of the `.app` at `path`.
fn bundle_identity(path: &Path) -> Option<AppIdentity> {
    let bundle = NSBundle::bundleWithPath(&NSString::from_str(path.to_str()?))?;
    let bundle_id = BundleId::try_from(bundle.bundleIdentifier()?.to_string()).ok()?;
    let name = path.file_stem().map_or_else(
        || bundle_id.to_string(),
        |stem| stem.to_string_lossy().into_owned(),
    );
    Some(AppIdentity { bundle_id, name })
}
