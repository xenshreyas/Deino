//! Cutting a meeting into chunks to embed (PARITY_PLAN §4.4).
//!
//! - **Transcript:** consecutive segments of one speaker form a turn; turns
//!   are packed into windows of about [`TARGET_TOKENS`], never cutting a
//!   turn unless it alone is too long (then it is split between segments).
//!   Each window after the first starts with the previous window's last turn
//!   when that is short ([`OVERLAP_TOKENS`]), so an answer that spans two
//!   windows is found in both. Lines read `Speaker: text`.
//! - **Notes and summaries:** one chunk per Markdown section (a heading and
//!   what follows it), split between paragraphs when longer than the target.
//!
//! Tokens are estimated at four characters each, which is close for English
//! with the nomic BERT tokenizer and errs on the long side.

use std::ops::Range;

use deino_domain::{MeetingId, Millis};

/// About how long a chunk is, in tokens.
pub const TARGET_TOKENS: usize = 400;
/// The longest turn repeated at the start of the next window, in tokens.
pub const OVERLAP_TOKENS: usize = 100;
/// The hard cap on a chunk's text, in characters, far inside the embedding
/// context (2,048 tokens) even for text that tokenizes badly.
pub const MAX_CHUNK_CHARS: usize = 4_000;
/// Characters per estimated token.
const CHARS_PER_TOKEN: usize = 4;

/// What a chunk was cut from (`embeddings.ref_kind`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, sqlx::Type)]
#[sqlx(rename_all = "snake_case")]
pub enum ChunkKind {
    /// A window of the transcript.
    Transcript,
    /// A section of the user's notes.
    Note,
    /// A section of the summary.
    Summary,
}

/// One transcript segment, as the chunker reads it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TranscriptLine {
    /// The segment's ID.
    pub segment_id: String,
    /// Where it starts in the recording.
    pub start: Millis,
    /// Where it ends.
    pub end: Millis,
    /// Who said it (a person's name or a speaker label), if known.
    pub speaker: Option<String>,
    /// What was said.
    pub text: String,
}

/// A piece of a meeting to embed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Chunk {
    /// What it was cut from.
    pub kind: ChunkKind,
    /// The window's first segment, or the meeting for a section.
    pub ref_id: String,
    /// Its position among the meeting's chunks of its kind.
    pub index: u32,
    /// The window's span in the recording (transcript only).
    pub span: Option<(Millis, Millis)>,
    /// The text to embed and show.
    pub text: String,
}

/// The estimated token count of `text`.
pub fn estimate_tokens(text: &str) -> usize {
    text.chars().count().div_ceil(CHARS_PER_TOKEN)
}

/// Turns, or pieces of overlong turns, as ranges of the input lines.
fn units(lines: &[TranscriptLine]) -> Vec<Range<usize>> {
    let mut units = Vec::new();
    let mut start = 0;
    let mut tokens = 0;
    for (index, line) in lines.iter().enumerate() {
        let line_tokens = estimate_tokens(&line.text);
        let new_turn = index > start
            && lines.get(index - 1).map(|previous| &previous.speaker) != Some(&line.speaker);
        if new_turn || (index > start && tokens + line_tokens > TARGET_TOKENS) {
            units.push(start..index);
            (start, tokens) = (index, 0);
        }
        tokens += line_tokens;
    }
    if start < lines.len() {
        units.push(start..lines.len());
    }
    units
}

/// The transcript's windows, in order.
pub fn transcript_chunks(lines: &[TranscriptLine]) -> Vec<Chunk> {
    let units = units(lines);
    let tokens = |unit: &Range<usize>| -> usize {
        lines.get(unit.clone()).map_or(0, |unit| {
            unit.iter().map(|line| estimate_tokens(&line.text)).sum()
        })
    };
    let mut windows: Vec<Vec<Range<usize>>> = Vec::new();
    let mut current: Vec<Range<usize>> = Vec::new();
    let mut fresh = false;
    for unit in units {
        let size: usize = current.iter().map(&tokens).sum();
        if fresh && size + tokens(&unit) > TARGET_TOKENS {
            let overlap = current
                .last()
                .filter(|last| tokens(last) <= OVERLAP_TOKENS)
                .cloned();
            windows.push(std::mem::replace(
                &mut current,
                overlap.into_iter().collect(),
            ));
        }
        current.push(unit);
        fresh = true;
    }
    if fresh {
        windows.push(current);
    }
    windows
        .iter()
        .zip(0_u32..)
        .filter_map(|(window, index)| window_chunk(lines, window, index))
        .collect()
}

/// One window as a chunk: `Speaker: text` per turn.
fn window_chunk(lines: &[TranscriptLine], window: &[Range<usize>], index: u32) -> Option<Chunk> {
    let first = lines.get(window.first()?.start)?;
    let last = lines.get(window.last()?.end.checked_sub(1)?)?;
    let text: Vec<String> = window
        .iter()
        .filter_map(|unit| lines.get(unit.clone()))
        .map(render_turn)
        .collect();
    Some(Chunk {
        kind: ChunkKind::Transcript,
        ref_id: first.segment_id.clone(),
        index,
        span: Some((first.start, last.end.max(first.start))),
        text: capped(&text.join("\n")),
    })
}

fn render_turn(turn: &[TranscriptLine]) -> String {
    let said: Vec<&str> = turn.iter().map(|line| line.text.trim()).collect();
    match turn.first().and_then(|line| line.speaker.as_deref()) {
        Some(speaker) => format!("{speaker}: {}", said.join(" ")),
        None => said.join(" "),
    }
}

/// The Markdown's sections as chunks of `kind`, referring to `meeting`.
pub fn section_chunks(kind: ChunkKind, meeting: &MeetingId, markdown: &str) -> Vec<Chunk> {
    sections(markdown)
        .into_iter()
        .flat_map(|section| split_long(&section))
        .zip(0_u32..)
        .map(|(text, index)| Chunk {
            kind,
            ref_id: meeting.as_str().to_owned(),
            index,
            span: None,
            text: capped(&text),
        })
        .collect()
}

/// The text from each heading to the next, without empty sections.
fn sections(markdown: &str) -> Vec<String> {
    let mut sections: Vec<String> = Vec::new();
    let mut current = String::new();
    for line in markdown.lines() {
        if line.trim_start().starts_with('#') && !current.trim().is_empty() {
            sections.push(std::mem::take(&mut current));
        }
        current.push_str(line);
        current.push('\n');
    }
    sections.push(current);
    sections
        .into_iter()
        .map(|section| section.trim().to_owned())
        .filter(|section| section.chars().any(char::is_alphanumeric))
        .collect()
}

/// A section longer than the target, split between paragraphs.
fn split_long(section: &str) -> Vec<String> {
    let mut pieces: Vec<String> = Vec::new();
    let mut current = String::new();
    for paragraph in section
        .split("\n\n")
        .map(str::trim)
        .filter(|p| !p.is_empty())
    {
        let fits = estimate_tokens(&current) + estimate_tokens(paragraph) <= TARGET_TOKENS;
        if !current.is_empty() && !fits {
            pieces.push(std::mem::take(&mut current));
        }
        if !current.is_empty() {
            current.push_str("\n\n");
        }
        current.push_str(paragraph);
    }
    if !current.is_empty() {
        pieces.push(current);
    }
    pieces
}

/// `text` cut to [`MAX_CHUNK_CHARS`] characters.
fn capped(text: &str) -> String {
    text.chars().take(MAX_CHUNK_CHARS).collect()
}

#[cfg(test)]
#[path = "chunk_tests.rs"]
mod tests;
