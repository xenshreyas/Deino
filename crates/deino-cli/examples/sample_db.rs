//! Writes the sample library (the eight fixture meetings plus tags,
//! folders, action items and AI notes) into a new database file, to try
//! `deino` and `deino mcp` without touching real data:
//!
//! ```sh
//! cargo run -p deino-cli --example sample_db -- /tmp/deino-sample.sqlite
//! cargo run -p deino-cli -- --db /tmp/deino-sample.sqlite meetings
//! ```

use std::path::PathBuf;
use std::process::ExitCode;

use deino_core::query::seed_sample_library;
use deino_db::{Database, DbLocation, MIGRATOR};
use deino_domain::SystemZone;

#[tokio::main]
async fn main() -> ExitCode {
    let Some(path) = std::env::args_os().nth(1).map(PathBuf::from) else {
        return report("usage: sample_db <new database file>");
    };
    if path.exists() {
        return report("refusing to overwrite an existing file");
    }
    let database = match Database::open(&DbLocation::File(path), &MIGRATOR).await {
        Ok(database) => database,
        Err(error) => return report(&error.to_string()),
    };
    let seeded = seed_sample_library(&database, &SystemZone).await;
    database.close().await;
    match seeded {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => report(&error.to_string()),
    }
}

#[allow(clippy::print_stderr, reason = "an example's only way to report")]
fn report(message: &str) -> ExitCode {
    eprintln!("sample_db: {message}");
    ExitCode::FAILURE
}
