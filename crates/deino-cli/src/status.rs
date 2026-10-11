//! `deino status`: which database the CLI reads, and whether it can.

use std::path::Path;

use deino_core::query::{MeetingFilter, MeetingSummary};
use serde::Serialize;

use crate::markdown::{line, when};
use crate::request::Markdown;
use crate::{CliError, Opened};

/// What `deino status` reports.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusReport {
    /// The database path.
    pub database: String,
    /// True when it opened read-only with a matching schema.
    pub readable: bool,
    /// Why not, when it did not.
    pub problem: Option<String>,
    /// The newest meeting, when there is one.
    pub latest_meeting: Option<MeetingSummary>,
}

/// Opens the database at `path` and reports on it; never fails.
pub async fn status(path: &Path) -> StatusReport {
    let latest = latest(path).await;
    StatusReport {
        database: path.display().to_string(),
        readable: latest.is_ok(),
        problem: latest.as_ref().err().map(ToString::to_string),
        latest_meeting: latest.ok().flatten(),
    }
}

async fn latest(path: &Path) -> Result<Option<MeetingSummary>, CliError> {
    let opened = Opened::read_only(path).await?;
    let filter = MeetingFilter {
        limit: 1,
        ..MeetingFilter::default()
    };
    Ok(opened
        .library
        .meetings
        .list(&filter)
        .await?
        .into_iter()
        .next())
}

impl Markdown for StatusReport {
    fn markdown(&self) -> String {
        let mut out = String::new();
        line(&mut out, format!("Database: {}", self.database));
        match (&self.problem, &self.latest_meeting) {
            (Some(problem), _) => line(&mut out, format!("Not readable: {problem}")),
            (None, Some(meeting)) => line(
                &mut out,
                format!(
                    "Readable. Latest meeting: {} ({})",
                    meeting.title,
                    when(&meeting.held_at)
                ),
            ),
            (None, None) => line(&mut out, "Readable. No meetings yet."),
        }
        out
    }
}
