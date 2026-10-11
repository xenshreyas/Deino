//! [`TranscriptQueries`]: a meeting's transcript, speaker-attributed and
//! timestamped, read a window at a time so a long meeting never has to be
//! read whole.
//!
//! Speakers are named exactly as exports name them (the person a voice was
//! identified as, "Me", or the diarization label), and segments the user
//! deleted are left out, both through the [`ExportAssembler`].

use deino_domain::{MeetingId, Millis};
use deino_export::Utterance;

use super::{QueryError, ReadPorts, TranscriptLine, TranscriptPage};
use crate::export::ExportAssembler;

/// How many lines a page holds when the caller does not say.
pub const DEFAULT_LINES: u32 = 300;
/// The most lines one page holds.
pub const MAX_LINES: u32 = 2_000;

/// Which part of a transcript to read: utterances starting in
/// `[from, to)`, at most `limit` of them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TranscriptWindow {
    /// Read from here; `None` is the start.
    pub from: Option<Millis>,
    /// Read up to here; `None` is the end.
    pub to: Option<Millis>,
    /// The most lines; 0 means [`DEFAULT_LINES`].
    pub limit: u32,
}

impl TranscriptWindow {
    fn limit(self) -> usize {
        let limit = match self.limit {
            0 => DEFAULT_LINES,
            limit => limit.min(MAX_LINES),
        };
        usize::try_from(limit).unwrap_or(usize::MAX)
    }

    fn admits(self, start: Millis) -> bool {
        self.from.is_none_or(|from| start >= from) && self.to.is_none_or(|to| start < to)
    }
}

/// Reads transcripts.
pub struct TranscriptQueries {
    ports: ReadPorts,
    assembler: ExportAssembler,
}

impl TranscriptQueries {
    /// The queries over `ports`.
    pub fn new(ports: ReadPorts) -> Self {
        let assembler = ExportAssembler::new(ports.meeting.clone());
        Self { ports, assembler }
    }

    /// The utterances of meeting `id` inside `window`; `next_from_ms` says
    /// where to continue when the page was cut at its limit.
    pub async fn page(
        &self,
        id: &MeetingId,
        window: TranscriptWindow,
    ) -> Result<TranscriptPage, QueryError> {
        if window
            .from
            .zip(window.to)
            .is_some_and(|(from, to)| from >= to)
        {
            return Err(QueryError::InvalidRange);
        }
        let meeting = self
            .ports
            .meeting
            .meetings
            .find(id)
            .await?
            .ok_or_else(|| QueryError::meeting_not_found(id))?;
        let utterances = self.assembler.transcript(id).await?;
        let total_lines = u32::try_from(utterances.len()).unwrap_or(u32::MAX);
        let mut inside = utterances
            .into_iter()
            .filter(|utterance| window.admits(utterance.start));
        let lines: Vec<TranscriptLine> = inside.by_ref().take(window.limit()).map(line).collect();
        Ok(TranscriptPage {
            meeting_id: meeting.id,
            title: meeting.title,
            next_from_ms: inside.next().map(|next| next.start.get()),
            total_lines,
            lines,
        })
    }
}

fn line(utterance: Utterance) -> TranscriptLine {
    TranscriptLine {
        start_ms: utterance.start.get(),
        end_ms: utterance.end.get(),
        speaker: utterance.speaker,
        is_me: utterance.is_me,
        text: utterance.text,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_window_is_half_open_and_clamped() {
        let window = TranscriptWindow {
            from: Some(Millis(1_000)),
            to: Some(Millis(2_000)),
            limit: 0,
        };

        assert!(window.admits(Millis(1_000)));
        assert!(!window.admits(Millis(2_000)));
        assert!(!window.admits(Millis(999)));
        assert_eq!(window.limit(), 300);
        assert_eq!(
            TranscriptWindow {
                limit: u32::MAX,
                ..window
            }
            .limit(),
            2_000
        );
    }
}
