//! `deino --dump-call-window <zoom|teams|meet> <file>`: saves every window of
//! a running meeting app as speaker-tag fixture JSON (the capture tool of
//! `crates/deino-speaker-tags/fixtures/README.md`), run as Deino so it uses
//! Deino's own Accessibility grant. Launch it through `LaunchServices`
//! (`open -n -a Deino --args --dump-call-window zoom <file>`), or macOS
//! checks the launching terminal's grant instead. The dump holds the call's
//! participant names; it is written only to the file given.

use std::ffi::OsString;

use deino_domain::CallApp;

/// The flag that selects the dump.
const FLAG: &str = "--dump-call-window";

/// Runs the dump when the arguments ask for it; returns the exit code.
pub fn run_if_requested(args: &[OsString]) -> Option<i32> {
    let at = args.iter().position(|arg| arg == FLAG)?;
    let app = args
        .get(at + 1)
        .and_then(|name| name.to_str())
        .and_then(app_named);
    let out = args.get(at + 2);
    let (Some(app), Some(out)) = (app, out) else {
        log::error!("usage: deino --dump-call-window <zoom|teams|meet> <file>");
        return Some(2);
    };
    Some(dump(app, out))
}

/// The meeting app called `name`.
fn app_named(name: &str) -> Option<CallApp> {
    CallApp::ALL
        .iter()
        .copied()
        .find(|app| app.as_str() == name)
}

/// Writes `app`'s windows to `out`.
fn dump(app: CallApp, out: &OsString) -> i32 {
    let granted = deino_speaker_tags::platform().is_some_and(|p| p.permission.granted());
    if !granted {
        log::error!("dump-call-window: Deino does not have Accessibility");
        return 1;
    }
    let written = deino_speaker_tags::capture(app)
        .map_err(|error| error.to_string())
        .and_then(|windows| serde_json::to_string_pretty(&windows).map_err(|e| e.to_string()))
        .and_then(|json| std::fs::write(out, json + "\n").map_err(|e| e.to_string()));
    match written {
        Ok(()) => 0,
        Err(error) => {
            log::error!("dump-call-window: {error}");
            1
        }
    }
}
