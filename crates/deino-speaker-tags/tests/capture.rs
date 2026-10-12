//! The capture tool behind `fixtures/README.md`: dumps every window of a
//! running meeting app as fixture JSON. Ignored by default; it reads the
//! app the user has open, so run it by hand during a call:
//!
//! ```bash
//! DEINO_AX_APP=zoom DEINO_AX_OUT=/tmp/zoom.json \
//!   cargo test -p deino-speaker-tags --test capture -- --ignored --nocapture
//! ```
//!
//! `DEINO_AX_APP` is `zoom`, `teams` or `meet`; without `DEINO_AX_OUT` the
//! dump goes to standard output. The terminal running it needs
//! Accessibility (System Settings → Privacy & Security → Accessibility);
//! the tool checks without prompting and stops if it is missing.

#![cfg(target_os = "macos")]
#![allow(
    clippy::unwrap_used,
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "a hand-run capture tool: it prints the dump and fails loudly"
)]

use deino_domain::CallApp;
use deino_speaker_tags::{capture, platform};

/// The app named by `DEINO_AX_APP` (Zoom by default).
fn app() -> CallApp {
    let named = std::env::var("DEINO_AX_APP").unwrap_or_default();
    CallApp::ALL
        .iter()
        .copied()
        .find(|app| app.as_str() == named)
        .unwrap_or(CallApp::Zoom)
}

#[test]
#[ignore = "reads a running meeting app's windows; needs Accessibility for the terminal"]
fn dump_a_running_meeting_app() {
    let granted = platform().is_some_and(|platform| platform.permission.granted());
    assert!(
        granted,
        "grant Accessibility to this terminal first; the tool never asks for it"
    );

    let windows = capture(app()).unwrap();
    let json = serde_json::to_string_pretty(&windows).unwrap();

    match std::env::var("DEINO_AX_OUT") {
        Ok(path) => std::fs::write(&path, json + "\n").unwrap(),
        Err(_) => println!("{json}"),
    }
    eprintln!("{} windows of {} dumped", windows.len(), app());
}
