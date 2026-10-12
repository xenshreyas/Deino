//! Loading the fixture dumps in `fixtures/<app>/<case>.json` and reading
//! them the way the watcher does.

use std::path::PathBuf;

use deino_domain::CallApp;
use deino_speaker_tags::{AxNode, ReadError, Reading, adapter_for};

/// The windows of one dump.
pub fn windows(app: CallApp, case: &str) -> Vec<AxNode> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(app.as_str())
        .join(format!("{case}.json"));
    let json = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    serde_json::from_str(&json).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

/// What the watcher would read from a dump: the first call window that
/// reads, else the first layout error, else "not in a call".
#[allow(
    dead_code,
    reason = "each test crate includes this module and uses part of it"
)]
pub fn read(app: CallApp, case: &str) -> Result<Reading, ReadError> {
    let adapter = adapter_for(app);
    let mut failure = ReadError::NotInCall;
    for window in windows(app, case)
        .iter()
        .filter(|window| adapter.is_call_window(&window.summary()))
    {
        match adapter.read(window) {
            Ok(reading) => return Ok(reading),
            Err(ReadError::NotInCall) => {}
            Err(error) => failure = error,
        }
    }
    Err(failure)
}
