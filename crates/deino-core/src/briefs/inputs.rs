//! What a brief is written from: the numbered sources, the deterministic
//! "enough to say?" check that runs before any model call, the prompt and
//! its fingerprint, and the validation of the model's answer.

use std::collections::BTreeMap;

use deino_llm::Prompt;
use deino_llm::brief::{BriefAnswer, SourceKind, SourceRef, source_block};
use deino_llm::prompts::{BriefSystem, BriefUser, PromptText};
use sha2::{Digest, Sha256};

use super::{BriefBullet, BriefCitation};

/// The fewest bullets a brief has; fewer valid ones mean no brief.
pub const MIN_BULLETS: usize = 2;
/// The most bullets a brief keeps.
pub const MAX_BULLETS: usize = 4;
/// The most citations one bullet keeps.
pub const MAX_CITATIONS: usize = 3;
/// The longest bullet kept, in characters (about 30 words, with room).
const MAX_BULLET_CHARS: usize = 300;
/// Lower-case hexadecimal digits, by value.
const HEX_DIGITS: &[u8; 16] = b"0123456789abcdef";
/// Room for four short bullets with their references.
const BRIEF_MAX_TOKENS: u32 = 1_024;

/// One thing the brief may cite.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Source {
    /// What a bullet citing it links to.
    pub citation: BriefCitation,
    /// Its one-line heading in the prompt.
    pub heading: String,
    /// What it says; may be empty (a meeting without notes).
    pub body: String,
}

impl Source {
    /// The kind its reference letter names.
    fn kind(&self) -> SourceKind {
        match self.citation {
            BriefCitation::Meeting { .. } => SourceKind::Meeting,
            BriefCitation::Moment { .. } | BriefCitation::ActionItem { .. } => {
                SourceKind::ActionItem
            }
            BriefCitation::Person { .. } => SourceKind::Person,
            BriefCitation::Company { .. } => SourceKind::Company,
            BriefCitation::Event => SourceKind::Event,
        }
    }
}

/// Everything one brief is written from.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BriefInputs {
    /// The event's title, time and attendees, one per line.
    pub event: String,
    /// The sources, meetings first.
    pub sources: Vec<Source>,
}

impl BriefInputs {
    /// Whether the user's own history says enough for a brief: an earlier
    /// meeting with notes or a summary, or two other facts (open items,
    /// person or company summaries). The invitation alone never counts, so
    /// a new contact gets no brief and no model call.
    pub fn is_sufficient(&self) -> bool {
        let meetings = self
            .sources
            .iter()
            .filter(|s| s.kind() == SourceKind::Meeting && !s.body.trim().is_empty())
            .count();
        let facts = self
            .sources
            .iter()
            .filter(|s| !matches!(s.kind(), SourceKind::Meeting | SourceKind::Event))
            .count();
        meetings >= 1 || facts >= 2
    }

    /// The numbered prompt, what each reference in it stands for, and its
    /// fingerprint.
    pub fn render(&self) -> Rendered {
        let mut counts: BTreeMap<SourceKind, u32> = BTreeMap::new();
        let mut index = SourceIndex::default();
        let mut blocks = Vec::with_capacity(self.sources.len());
        for source in &self.sources {
            let count = counts.entry(source.kind()).or_default();
            *count += 1;
            let reference = SourceRef {
                kind: source.kind(),
                number: *count,
            };
            blocks.push(source_block(reference, &source.heading, &source.body));
            index
                .by_reference
                .insert(reference, source.citation.clone());
        }
        let user = BriefUser {
            event: &self.event,
            sources: &blocks.join("\n\n"),
        }
        .text();
        let system = BriefSystem.text();
        let fingerprint = fingerprint(&system, &user);
        Rendered {
            prompt: Prompt::new(user)
                .system(system)
                .max_tokens(BRIEF_MAX_TOKENS),
            index,
            fingerprint,
        }
    }
}

/// A rendered brief prompt.
#[derive(Clone, Debug)]
pub struct Rendered {
    /// What the model is asked.
    pub prompt: Prompt,
    /// What its references stand for.
    pub index: SourceIndex,
    /// The SHA-256 (hex) of its text: the inputs' fingerprint.
    pub fingerprint: String,
}

/// The SHA-256 (hex) of a prompt's system and user text, which changes
/// exactly when what the model would see does.
fn fingerprint(system: &str, user: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(system.as_bytes());
    hasher.update([0]);
    hasher.update(user.as_bytes());
    hasher
        .finalize()
        .iter()
        .flat_map(|byte| [byte >> 4, byte & 0x0f])
        .map(|nibble| {
            HEX_DIGITS
                .get(usize::from(nibble))
                .map_or('0', |digit| char::from(*digit))
        })
        .collect()
}

/// What each reference of one prompt stands for.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SourceIndex {
    by_reference: BTreeMap<SourceRef, BriefCitation>,
}

impl SourceIndex {
    /// The citations `references` name, in order, without repeats or
    /// unknown references, at most [`MAX_CITATIONS`].
    pub fn resolve(&self, references: &[String]) -> Vec<BriefCitation> {
        let mut resolved: Vec<BriefCitation> = Vec::new();
        let known = references
            .iter()
            .filter_map(|reference| SourceRef::parse(reference))
            .filter_map(|reference| self.by_reference.get(&reference));
        for citation in known {
            if !resolved.contains(citation) {
                resolved.push(citation.clone());
            }
        }
        resolved.truncate(MAX_CITATIONS);
        resolved
    }

    /// The model's answer as bullets: each keeps only citations of real
    /// sources, a bullet without any is dropped, at most [`MAX_BULLETS`]
    /// are kept. `None` when the model said there is too little, or fewer
    /// than [`MIN_BULLETS`] survive.
    pub fn bullets(&self, answer: BriefAnswer) -> Option<Vec<BriefBullet>> {
        if answer.insufficient {
            return None;
        }
        let bullets: Vec<BriefBullet> = answer
            .bullets
            .into_iter()
            .filter_map(|bullet| {
                let text: String = bullet.text.trim().chars().take(MAX_BULLET_CHARS).collect();
                let citations = self.resolve(&bullet.sources);
                (!text.is_empty() && !citations.is_empty())
                    .then_some(BriefBullet { text, citations })
            })
            .take(MAX_BULLETS)
            .collect();
        (bullets.len() >= MIN_BULLETS).then_some(bullets)
    }
}

#[cfg(test)]
#[path = "inputs_tests.rs"]
mod tests;
