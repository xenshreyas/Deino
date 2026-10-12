//! A block's new text back into inline content, keeping what it kept.
//!
//! The block is shown to the model as words with its citations as transcript
//! references (`[s12]`) and its mentions as `@Name`. The model's new text is
//! split the same way and aligned with the old words (longest common
//! subsequence): a kept word keeps its run's marks (its origin, bold, a
//! link), a kept citation or mention stays the same node, and every new word
//! is AI text. New references resolve to citations of real transcript lines;
//! unknown ones are dropped. Citations the new text left out are put back at
//! its end, so a rewrite never loses evidence.

use std::collections::BTreeMap;
use std::sync::LazyLock;

use deino_doc::{CitationAttrs, Inline, Mark, OriginAttrs};
use deino_domain::SegmentId;
use regex::Regex;

use super::words::{common, split_words};
use crate::enhance::TranscriptLine;

/// A transcript reference in the model's text: `[s12]`.
static REFERENCE: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"\[\s*[sS](\d+)\s*\]").ok());

/// The meeting's transcript references, both ways.
#[derive(Clone, Debug, Default)]
pub struct Refs {
    by_segment: BTreeMap<SegmentId, u32>,
    by_number: BTreeMap<u32, CitationAttrs>,
}

impl Refs {
    /// The references of `lines`.
    pub fn new(lines: &[TranscriptLine]) -> Self {
        let mut refs = Self::default();
        for line in lines {
            refs.by_segment.insert(line.segment_id, line.reference);
            let attrs = CitationAttrs {
                segment_id: line.segment_id,
                start_ms: line.start,
            };
            refs.by_number.insert(line.reference, attrs);
        }
        refs
    }

    /// `[s12]` for a citation of a line still in the transcript.
    fn label(&self, attrs: &CitationAttrs) -> Option<String> {
        self.by_segment
            .get(&attrs.segment_id)
            .map(|number| format!("[s{number}]"))
    }
}

/// One word, space or node of a block, as the model sees it.
#[derive(Clone, Debug)]
struct Token {
    shown: String,
    inline: Option<Inline>,
}

/// The block's text as the prompt shows it.
pub fn shown_text(content: &[Inline], refs: &Refs) -> String {
    tokens(content, refs)
        .iter()
        .map(|token| token.shown.as_str())
        .collect::<String>()
        .trim()
        .to_owned()
}

/// The inline content of a block whose text became `text`.
pub fn rewrite(original: &[Inline], text: &str, refs: &Refs) -> Vec<Inline> {
    let old = tokens(original, refs);
    let mentions: Vec<&Token> = old
        .iter()
        .filter(|token| matches!(token.inline, Some(Inline::Mention { .. })))
        .collect();
    let new = parse(text.trim(), &mentions, refs);
    let old_shown: Vec<&str> = old.iter().map(|token| token.shown.as_str()).collect();
    let new_shown: Vec<&str> = new.iter().map(|token| token.shown.as_str()).collect();
    let kept: BTreeMap<usize, usize> = common(&old_shown, &new_shown)
        .into_iter()
        .map(|(i, j)| (j, i))
        .collect();
    let mut content: Vec<Inline> = new
        .into_iter()
        .enumerate()
        .filter_map(|(j, token)| match kept.get(&j).and_then(|&i| old.get(i)) {
            Some(kept) => kept.inline.clone(),
            None => token.inline,
        })
        .collect();
    restore_citations(original, &mut content);
    content
}

/// The tokens of existing content: text split into words with their marks,
/// nodes whole. Citations of lines no longer in the transcript are hidden.
fn tokens(content: &[Inline], refs: &Refs) -> Vec<Token> {
    let mut tokens = Vec::new();
    for inline in content {
        match inline {
            Inline::Text { text, marks } => {
                tokens.extend(split_words(text).into_iter().map(|word| Token {
                    shown: word.to_owned(),
                    inline: Some(Inline::text(word, marks.clone())),
                }));
            }
            Inline::HardBreak { .. } => tokens.push(node("\n", inline)),
            Inline::Mention { attrs, .. } => {
                tokens.push(node(&format!("@{}", attrs.label), inline))
            }
            Inline::Citation { attrs, .. } => {
                if let Some(label) = refs.label(attrs) {
                    tokens.push(node(&label, inline));
                }
            }
        }
    }
    tokens
}

fn node(shown: &str, inline: &Inline) -> Token {
    Token {
        shown: shown.to_owned(),
        inline: Some(inline.clone()),
    }
}

/// The model's text as tokens: references, the block's mentions, words.
fn parse(text: &str, mentions: &[&Token], refs: &Refs) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut last = 0;
    let found = REFERENCE
        .as_ref()
        .map(|pattern| pattern.captures_iter(text));
    for captures in found.into_iter().flatten() {
        let (Some(whole), Some(number)) = (captures.get(0), captures.get(1)) else {
            continue;
        };
        words(
            text.get(last..whole.start()).unwrap_or_default(),
            mentions,
            &mut tokens,
        );
        let attrs = number
            .as_str()
            .parse()
            .ok()
            .and_then(|n| refs.by_number.get(&n));
        if let Some(attrs) = attrs {
            tokens.push(Token {
                shown: format!("[s{}]", number.as_str()),
                inline: Some(Inline::Citation {
                    attrs: *attrs,
                    marks: ai(),
                }),
            });
        }
        last = whole.end();
    }
    words(text.get(last..).unwrap_or_default(), mentions, &mut tokens);
    tokens
}

/// `text` as words, with the block's mentions (`@Name`) kept whole.
fn words(text: &str, mentions: &[&Token], tokens: &mut Vec<Token>) {
    let mut rest = text;
    while !rest.is_empty() {
        let next = mentions
            .iter()
            .filter_map(|mention| rest.find(&mention.shown).map(|at| (at, *mention)))
            .min_by_key(|(at, _)| *at);
        let Some((at, mention)) = next else {
            break;
        };
        push_words(rest.get(..at).unwrap_or_default(), tokens);
        tokens.push(mention.clone());
        rest = rest.get(at + mention.shown.len()..).unwrap_or_default();
    }
    push_words(rest, tokens);
}

fn push_words(text: &str, tokens: &mut Vec<Token>) {
    tokens.extend(split_words(text).into_iter().map(|word| Token {
        shown: word.to_owned(),
        inline: Some(Inline::text(word, ai())),
    }));
}

/// Appends the citations of `original` that `content` lost.
fn restore_citations(original: &[Inline], content: &mut Vec<Inline>) {
    let cited = |inlines: &[Inline]| -> Vec<CitationAttrs> {
        inlines
            .iter()
            .filter_map(|inline| match inline {
                Inline::Citation { attrs, .. } => Some(*attrs),
                _ => None,
            })
            .collect()
    };
    let kept = cited(content);
    let lost: Vec<&Inline> = original
        .iter()
        .filter(|inline| matches!(inline, Inline::Citation { attrs, .. } if !kept.contains(attrs)))
        .collect();
    if lost.is_empty() {
        return;
    }
    let ends_in_space =
        matches!(content.last(), Some(Inline::Text { text, .. }) if text.ends_with(' '));
    if !ends_in_space && !content.is_empty() {
        content.push(Inline::text(" ", ai()));
    }
    content.extend(lost.into_iter().cloned());
}

fn ai() -> Vec<Mark> {
    vec![Mark::Origin {
        attrs: OriginAttrs::AI,
    }]
}

#[cfg(test)]
mod rewrite_tests;
