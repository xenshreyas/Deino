//! The map step of a long transcript: each part noted on its own, in
//! parallel, a part that failed noted in halves. Parts only report how many
//! are done; their notes are not the notes yet.

use deino_llm::LlmError;
use futures_util::{StreamExt, stream};

use super::fallback::{halve, is_answer_failure};
use super::{NotesWriter, Pass, concurrency};
use crate::enhance::transcript::TranscriptLine;
use crate::enhance::wire::EnhancedDoc;

/// Part `number` (from 1) of `count` of a long meeting's transcript.
#[derive(Clone, Copy)]
struct Part<'a> {
    number: usize,
    count: usize,
    lines: &'a [TranscriptLine],
}

/// The notes of some transcript lines.
type Noted<'a> = (&'a [TranscriptLine], EnhancedDoc);

impl NotesWriter {
    /// The map step: each part's notes, in order, a part that failed noted
    /// in halves; at least one part must be noted.
    pub(super) async fn map<'a>(
        &self,
        pass: &Pass<'_>,
        cuts: &[&'a [TranscriptLine]],
    ) -> Result<Vec<Noted<'a>>, LlmError> {
        let count = cuts.len();
        pass.report.parts(0, count);
        // Collected first: a lazy iterator's closure inside the stream would
        // make the future `Send` only for one lifetime (rust-lang/rust#102211).
        let calls: Vec<_> = cuts
            .iter()
            .enumerate()
            .map(|(at, lines)| {
                let part = Part {
                    number: at + 1,
                    count,
                    lines,
                };
                self.note_part(pass, part)
            })
            .collect();
        let mut parts = stream::iter(calls).buffered(concurrency(&pass.target));
        let mut noted = Vec::new();
        let mut done = 0;
        while let Some(part) = parts.next().await {
            noted.extend(part?);
            done += 1;
            pass.report.parts(done, count);
        }
        if noted.is_empty() {
            return Err(LlmError::InvalidOutput {
                detail: "no part of the transcript could be noted".to_owned(),
            });
        }
        Ok(noted)
    }

    /// A part's notes, or its halves' notes when its answer could not be
    /// used twice; a half that fails is left out.
    async fn note_part<'a>(
        &self,
        pass: &Pass<'_>,
        part: Part<'a>,
    ) -> Result<Vec<Noted<'a>>, LlmError> {
        let Part {
            number,
            count,
            lines,
        } = part;
        let prompt = pass.inputs.part(number, count, lines);
        match self.ask_again(prompt, &pass.target, None).await {
            Ok(whole) => return Ok(vec![(lines, whole)]),
            Err(error) if !is_answer_failure(&error) => return Err(error),
            Err(error) => {
                log::warn!(target: "enhance", "part {number} of {count} failed ({:?}); noting it in halves", error.kind());
            }
        }
        let mut kept = Vec::new();
        for half in halve(lines) {
            let prompt = pass.inputs.part(number, count, half);
            match self.ask(prompt, &pass.target, None).await {
                Ok(half_notes) => kept.push((half, half_notes)),
                Err(error) if is_answer_failure(&error) => {
                    log::warn!(target: "enhance", "half of part {number} of {count} left out ({:?})", error.kind());
                }
                Err(error) => return Err(error),
            }
        }
        Ok(kept)
    }
}
