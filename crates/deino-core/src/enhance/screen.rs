//! The shared screen beside the transcript (CAP-09): each read snapshot
//! becomes a `[screen 12:04] …` line placed among the transcript lines at
//! its time, so the notes use a slide's text where it was discussed.
//!
//! A snapshot's text is flattened to one line (its lines joined by ` · `),
//! its description first; a text repeating the previous one (the same
//! slide with a participant's tile changed) is dropped; each line and the
//! whole are capped so the screen never crowds out the transcript.

use deino_domain::Millis;

use super::transcript::{TranscriptLine, clock};
use crate::screen::ScreenText;

/// The most of one snapshot a prompt carries, in characters.
pub const MAX_SCREEN_LINE_CHARS: usize = 1_500;
/// The most screen text a prompt carries in all, in characters.
pub const MAX_SCREEN_CHARS: usize = 12_000;

/// One snapshot as the prompt shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScreenLine {
    /// Where on the recording's timeline it was taken.
    pub at: Millis,
    /// Its description and text, on one line.
    pub text: String,
}

impl ScreenLine {
    /// `[screen 12:04] text`.
    pub fn render(&self) -> String {
        format!("[screen {}] {}", clock(self.at), self.text)
    }
}

/// The prompt's screen lines from the stored `texts` (see the module docs).
pub fn screen_lines(texts: &[ScreenText]) -> Vec<ScreenLine> {
    let mut lines: Vec<ScreenLine> = Vec::new();
    let mut total = 0;
    for text in texts {
        let flat = flatten(text);
        if flat.is_empty() || lines.last().is_some_and(|last| last.text == flat) {
            continue;
        }
        total += flat.chars().count();
        if total > MAX_SCREEN_CHARS {
            break;
        }
        lines.push(ScreenLine {
            at: text.at,
            text: flat,
        });
    }
    lines
}

/// The description, then the text, on one capped line.
fn flatten(text: &ScreenText) -> String {
    let parts = text
        .description
        .iter()
        .flat_map(|description| description.lines())
        .chain(text.text.lines())
        .map(str::trim)
        .filter(|line| !line.is_empty());
    let joined = parts.collect::<Vec<_>>().join(" · ");
    joined.chars().take(MAX_SCREEN_LINE_CHARS).collect()
}

/// The transcript `lines` with the `screen` lines placed by time: each
/// before the first transcript line that starts after it.
pub fn render_with_screen(lines: &[TranscriptLine], screen: &[ScreenLine]) -> String {
    let mut out = Vec::with_capacity(lines.len() + screen.len());
    let mut pending = screen.iter().peekable();
    for line in lines {
        while let Some(shown) = pending.next_if(|shown| shown.at <= line.start) {
            out.push(shown.render());
        }
        out.push(line.render());
    }
    out.extend(pending.map(ScreenLine::render));
    out.join("\n")
}

/// The `screen` lines that belong to a part of `lines`: from the part's
/// start (the meeting's, for the first) to its end (the meeting's, for the
/// last).
pub fn within(
    screen: &[ScreenLine],
    lines: &[TranscriptLine],
    (first, last): (bool, bool),
) -> Vec<ScreenLine> {
    let from = lines
        .first()
        .filter(|_| !first)
        .map_or(Millis(0), |line| line.start);
    let to = lines.last().filter(|_| !last).map(|line| line.end);
    screen
        .iter()
        .filter(|shown| shown.at >= from && to.is_none_or(|to| shown.at < to))
        .cloned()
        .collect()
}

#[cfg(test)]
#[path = "screen/screen_tests.rs"]
mod tests;
