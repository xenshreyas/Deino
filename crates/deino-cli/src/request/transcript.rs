//! `get_transcript` / `deino transcripts read <id>`.

use deino_core::query::{Library, TranscriptPage, TranscriptWindow};
use deino_domain::Millis;
use schemars::JsonSchema;
use serde::Deserialize;

use super::{Request, meeting_id};
use crate::CliError;

/// Read a meeting's transcript, a time window at a time.
#[derive(Clone, Debug, Deserialize, JsonSchema, clap::Args)]
#[serde(deny_unknown_fields)]
pub struct GetTranscript {
    /// The meeting's ID.
    pub meeting_id: String,
    /// Start of the window, in milliseconds from the start of the recording.
    #[arg(long)]
    #[serde(default)]
    pub from_ms: Option<u32>,
    /// End of the window (exclusive), in milliseconds.
    #[arg(long)]
    #[serde(default)]
    pub to_ms: Option<u32>,
    /// The most lines to return (default 300, at most 2000). The terminal
    /// prints every line when this is not given.
    #[arg(long, short = 'n')]
    #[serde(default)]
    pub limit: Option<u32>,
}

impl GetTranscript {
    fn window(&self) -> TranscriptWindow {
        TranscriptWindow {
            from: self.from_ms.map(Millis),
            to: self.to_ms.map(Millis),
            limit: self.limit.unwrap_or_default(),
        }
    }

    /// Every line in the window, page after page: what the terminal shows
    /// when no limit is given.
    pub async fn read_all(self, library: &Library) -> Result<TranscriptPage, CliError> {
        let id = meeting_id(&self.meeting_id)?;
        let mut window = TranscriptWindow {
            limit: deino_core::query::MAX_LINES,
            ..self.window()
        };
        let mut page = library.transcripts.page(&id, window).await?;
        while let Some(next) = page.next_from_ms {
            window.from = Some(Millis(next));
            let more = library.transcripts.page(&id, window).await?;
            page.lines.extend(more.lines);
            page.next_from_ms = more.next_from_ms;
        }
        Ok(page)
    }
}

impl Request for GetTranscript {
    const NAME: &'static str = "get_transcript";
    const TITLE: &'static str = "Get a transcript";
    const DESCRIPTION: &'static str = "Read what was said in a meeting: speaker-attributed, \
        timestamped lines in time order (deleted lines are left out). Long meetings come in \
        pages of at most `limit` lines (default 300): when nextFromMs is set, call again with \
        from_ms = nextFromMs to continue. Use from_ms/to_ms (milliseconds from the start) to \
        read only part, for example around a moment from search. Each line's start is also a \
        link: deino://meeting/<id>?t=<startMs>. Prefer read_meeting's notes for summaries; use \
        this for exact wording, quotes and who said what.";
    type Output = TranscriptPage;

    async fn run(self, library: &Library) -> Result<TranscriptPage, CliError> {
        let id = meeting_id(&self.meeting_id)?;
        Ok(library.transcripts.page(&id, self.window()).await?)
    }
}
