//! The macOS adapters behind [`crate::ports`]: Core Audio process objects
//! and the default input device (cidre), cameras (CoreMediaIO), running apps
//! (`NSWorkspace`) and browser windows (Accessibility).

mod apps;
mod browser;
mod camera;
mod cmio;
mod input;
mod listener;
mod processes;

use crate::model::Pid;
use crate::ports::Sensors;

/// The sensors of this Mac.
pub(crate) fn sensors() -> Sensors {
    Sensors {
        processes: Box::new(processes::CoreAudioProcesses),
        microphone: Box::new(input::DefaultInput),
        camera: Box::new(camera::Cameras),
        apps: Box::new(apps::Workspace),
        tabs: Box::new(browser::AxBrowserTabs),
        own_pid: Pid(own_pid()),
    }
}

/// Whether Accessibility is granted.
pub(crate) fn reads_browser_urls() -> bool {
    cidre::ax::is_process_trusted()
}

/// This process's id. macOS pids fit in an `i32` (`pid_t`).
fn own_pid() -> i32 {
    i32::try_from(std::process::id()).unwrap_or(i32::MAX)
}
