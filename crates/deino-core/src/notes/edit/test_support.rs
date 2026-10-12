//! Terse builders for this module's tests: documents with origin marks,
//! citations, mentions and links, and transcript lines to cite.

use deino_doc::{
    Block, CitationAttrs, Doc, HeadingAttrs, Inline, LinkAttrs, ListItem, Mark, MentionAttrs,
    OriginAttrs,
};
use deino_domain::{Millis, PersonId, SegmentId};

use crate::enhance::TranscriptLine;

/// Niall, the person mentioned in the test notes.
pub const NIALL: &str = "0192f3a0-6c1e-7b2a-9d4e-1f2a3b4c5d6e";

/// The ID of transcript segment `reference`.
pub fn segment(reference: u32) -> SegmentId {
    uuid::Uuid::from_u128(0x0192_f3a0_6c1e_7b2a_9d4e_0000_0000_0000 + u128::from(reference))
        .to_string()
        .parse()
        .unwrap()
}

/// Transcript line `reference` at `seconds`.
pub fn line(reference: u32, seconds: u32, text: &str) -> TranscriptLine {
    TranscriptLine {
        reference,
        segment_id: segment(reference),
        start: Millis(seconds * 1_000),
        end: Millis(seconds * 1_000 + 4_000),
        speaker: "Maya".into(),
        text: text.into(),
    }
}

/// Text the user typed.
pub fn user(text: &str) -> Inline {
    Inline::text(text, vec![origin(OriginAttrs::USER)])
}

/// Text the AI wrote.
pub fn ai(text: &str) -> Inline {
    Inline::text(text, vec![origin(OriginAttrs::AI)])
}

/// AI text linked to `href`.
pub fn ai_link(text: &str, href: &str) -> Inline {
    let link = Mark::Link {
        attrs: LinkAttrs {
            href: href.into(),
            title: None,
        },
    };
    Inline::text(text, vec![origin(OriginAttrs::AI), link])
}

/// An AI citation of segment `reference`.
pub fn cite(reference: u32) -> Inline {
    Inline::Citation {
        attrs: CitationAttrs {
            segment_id: segment(reference),
            start_ms: Millis(reference * 1_000),
        },
        marks: vec![origin(OriginAttrs::AI)],
    }
}

/// The user's mention of Niall, labelled `label`.
pub fn mention(label: &str) -> Inline {
    Inline::Mention {
        attrs: MentionAttrs {
            id: NIALL.parse::<PersonId>().unwrap(),
            label: label.into(),
        },
        marks: vec![origin(OriginAttrs::USER)],
    }
}

/// A level-3 AI heading.
pub fn heading(text: &str) -> Block {
    Block::Heading {
        attrs: HeadingAttrs { level: 3 },
        content: vec![ai(text)],
    }
}

/// A paragraph.
pub fn para(content: Vec<Inline>) -> Block {
    Block::Paragraph { content }
}

/// A bullet list of one paragraph per item.
pub fn bullets(items: Vec<Vec<Inline>>) -> Block {
    Block::BulletList {
        content: items
            .into_iter()
            .map(|content| ListItem {
                content: vec![para(content)],
            })
            .collect(),
    }
}

/// A document of `content`.
pub fn doc(content: Vec<Block>) -> Doc {
    Doc { content }
}

fn origin(attrs: OriginAttrs) -> Mark {
    Mark::Origin { attrs }
}
