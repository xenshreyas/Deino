//! Terse document builders for this crate's tests.

use deino_domain::{Millis, PersonId, SegmentId};

use crate::{
    Block, CitationAttrs, Doc, HeadingAttrs, ImageAttrs, Inline, LinkAttrs, ListItem, Mark,
    MentionAttrs, OrderedListAttrs, OriginAttrs, TaskItem, TaskItemAttrs,
};

pub(crate) const PRIYA: &str = "0192f3a0-6c1e-7b2a-9d4e-1f2a3b4c5d6e";
pub(crate) const SEGMENT: &str = "0192f3a0-6c1e-7b2a-9d4e-aaaaaaaaaaaa";

pub(crate) fn doc(content: Vec<Block>) -> Doc {
    Doc { content }
}

pub(crate) fn p(content: Vec<Inline>) -> Block {
    Block::Paragraph { content }
}

pub(crate) fn h(level: u8, content: Vec<Inline>) -> Block {
    Block::Heading {
        attrs: HeadingAttrs { level },
        content,
    }
}

pub(crate) fn t(text: &str) -> Inline {
    Inline::text(text, vec![])
}

pub(crate) fn m(text: &str, marks: Vec<Mark>) -> Inline {
    Inline::text(text, marks)
}

pub(crate) fn br() -> Inline {
    Inline::HardBreak { marks: vec![] }
}

pub(crate) fn link(href: &str) -> Mark {
    Mark::Link {
        attrs: LinkAttrs {
            href: href.into(),
            title: None,
        },
    }
}

pub(crate) fn ai() -> Mark {
    Mark::Origin {
        attrs: OriginAttrs::AI,
    }
}

pub(crate) fn user() -> Mark {
    Mark::Origin {
        attrs: OriginAttrs::USER,
    }
}

pub(crate) fn ai_edited() -> Mark {
    Mark::Origin {
        attrs: OriginAttrs {
            edited: true,
            ..OriginAttrs::AI
        },
    }
}

pub(crate) fn mention(label: &str, marks: Vec<Mark>) -> Inline {
    Inline::Mention {
        attrs: MentionAttrs {
            id: PRIYA.parse::<PersonId>().unwrap(),
            label: label.into(),
        },
        marks,
    }
}

pub(crate) fn citation(start_ms: u32, marks: Vec<Mark>) -> Inline {
    Inline::Citation {
        attrs: CitationAttrs {
            segment_id: SEGMENT.parse::<SegmentId>().unwrap(),
            start_ms: Millis(start_ms),
        },
        marks,
    }
}

pub(crate) fn bullets(items: Vec<Vec<Block>>) -> Block {
    Block::BulletList {
        content: items
            .into_iter()
            .map(|content| ListItem { content })
            .collect(),
    }
}

pub(crate) fn numbered(start: u32, items: Vec<Vec<Block>>) -> Block {
    Block::OrderedList {
        attrs: OrderedListAttrs { start },
        content: items
            .into_iter()
            .map(|content| ListItem { content })
            .collect(),
    }
}

pub(crate) fn tasks(items: Vec<(bool, Vec<Block>)>) -> Block {
    Block::TaskList {
        content: items
            .into_iter()
            .map(|(checked, content)| TaskItem {
                attrs: TaskItemAttrs { checked },
                content,
            })
            .collect(),
    }
}

pub(crate) fn image(src: &str, alt: Option<&str>, title: Option<&str>) -> Block {
    Block::Image {
        attrs: ImageAttrs {
            src: src.into(),
            alt: alt.map(Into::into),
            title: title.map(Into::into),
        },
    }
}
