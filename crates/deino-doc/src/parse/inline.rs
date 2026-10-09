//! Phrasing mdast nodes → [`Inline`]s, with marks exactly as nested.
//!
//! Marks come from two places: the mdast nesting (`Strong` inside `Link`)
//! and the inline HTML tags still open at that point (`<u>`, origin spans),
//! which mdast reports as separate open and close nodes.

use markdown::mdast::{self, Node};

use super::block::image_attrs;
use crate::html::{Tag, parse_tag};
use crate::validate::is_safe_href;
use crate::{ImageAttrs, Inline, LinkAttrs, Mark};

/// A piece of phrasing content: inline, or an image to lift into a block.
pub(super) enum Segment {
    /// Inline content.
    Inline(Inline),
    /// An image, which the editor shows as a block.
    Image(ImageAttrs),
}

impl Segment {
    /// The inline content, dropping images.
    pub(super) fn into_inline(self) -> Option<Inline> {
        match self {
            Self::Inline(inline) => Some(inline),
            Self::Image(_) => None,
        }
    }
}

/// The segments of a run of phrasing nodes, with adjacent equal text merged.
pub(super) fn segments(nodes: &[Node]) -> Vec<Segment> {
    let mut builder = Builder::default();
    builder.nodes(nodes, &[]);
    builder.out
}

#[derive(Default)]
struct Builder {
    out: Vec<Segment>,
    /// Marks opened by HTML tags that have not been closed yet.
    open_html: Vec<Mark>,
}

impl Builder {
    fn nodes(&mut self, nodes: &[Node], marks: &[Mark]) {
        for node in nodes {
            self.node(node, marks);
        }
    }

    fn node(&mut self, node: &Node, marks: &[Mark]) {
        match node {
            Node::Text(text) => self.text(&text.value, marks),
            Node::Strong(strong) => self.nodes(&strong.children, &with(marks, Mark::Bold)),
            Node::Emphasis(emphasis) => self.nodes(&emphasis.children, &with(marks, Mark::Italic)),
            Node::Delete(delete) => self.nodes(&delete.children, &with(marks, Mark::Strike)),
            Node::InlineCode(code) => self.text(&code.value, &with(marks, Mark::Code)),
            Node::InlineMath(math) => self.text(&math.value, &with(marks, Mark::Code)),
            Node::Link(link) => self.link(link, marks),
            Node::Break(_) => self.push(Inline::HardBreak {
                marks: self.effective(marks),
            }),
            Node::Html(html) => self.html(&html.value, marks),
            Node::Image(image) => self.out.push(Segment::Image(image_attrs(image))),
            Node::ImageReference(image) => self.text(&image.alt, marks),
            Node::FootnoteReference(_) => {}
            other => match other.children() {
                Some(children) => self.nodes(children, marks),
                None => self.text(&other.to_string(), marks),
            },
        }
    }

    /// A link keeps its target only when it is safe to open; otherwise its
    /// text stays, unlinked.
    fn link(&mut self, link: &mdast::Link, marks: &[Mark]) {
        if !is_safe_href(&link.url) {
            return self.nodes(&link.children, marks);
        }
        let attrs = LinkAttrs {
            href: link.url.clone(),
            title: link.title.clone(),
        };
        self.nodes(&link.children, &with(marks, Mark::Link { attrs }));
    }

    /// Text, with each soft line break turned into a hard break so the
    /// author's lines survive (CommonMark trims the spaces around it).
    fn text(&mut self, value: &str, marks: &[Mark]) {
        let marks = self.effective(marks);
        let lines: Vec<&str> = value.split('\n').collect();
        let last = lines.len().saturating_sub(1);
        for (index, mut line) in lines.into_iter().enumerate() {
            if index > 0 {
                self.push(Inline::HardBreak {
                    marks: marks.clone(),
                });
                line = line.trim_start();
            }
            if index < last {
                line = line.trim_end();
            }
            if !line.is_empty() {
                self.push(Inline::text(line, marks.clone()));
            }
        }
    }

    fn html(&mut self, raw: &str, marks: &[Mark]) {
        match parse_tag(raw) {
            Tag::Open(mark) => self.open_html.push(mark),
            Tag::Close(mark) => {
                if let Some(index) = self
                    .open_html
                    .iter()
                    .rposition(|open| open.same_kind(&mark))
                {
                    self.open_html.remove(index);
                }
            }
            Tag::Atom(atom) => self.push(with_marks(atom, self.effective(marks))),
            Tag::Break => self.push(Inline::HardBreak {
                marks: self.effective(marks),
            }),
            Tag::Ignored => {}
            Tag::Other => self.text(raw, marks),
        }
    }

    /// The marks in force: open HTML marks, then the nesting's, one per kind
    /// (the innermost wins), sorted by rank.
    fn effective(&self, marks: &[Mark]) -> Vec<Mark> {
        let mut effective: Vec<Mark> = Vec::new();
        for mark in self.open_html.iter().chain(marks) {
            effective.retain(|kept| !kept.same_kind(mark));
            effective.push(mark.clone());
        }
        effective.sort_by_key(Mark::rank);
        effective
    }

    /// Appends `inline`, merging it into the previous run of equal text.
    fn push(&mut self, inline: Inline) {
        if let (
            Some(Segment::Inline(Inline::Text { text, marks })),
            Inline::Text {
                text: more,
                marks: more_marks,
            },
        ) = (self.out.last_mut(), &inline)
            && marks == more_marks
        {
            text.push_str(more);
            return;
        }
        self.out.push(Segment::Inline(inline));
    }
}

/// `marks` plus one more.
fn with(marks: &[Mark], mark: Mark) -> Vec<Mark> {
    let mut marks = marks.to_vec();
    marks.push(mark);
    marks
}

/// An atom with its marks set.
fn with_marks(atom: Inline, marks: Vec<Mark>) -> Inline {
    match atom {
        Inline::Mention { attrs, .. } => Inline::Mention { attrs, marks },
        Inline::Citation { attrs, .. } => Inline::Citation { attrs, marks },
        other => other,
    }
}
