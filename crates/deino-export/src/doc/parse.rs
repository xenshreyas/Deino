//! Markdown → [`Block`]s, over pulldown-cmark's event stream.
//!
//! The parser keeps a stack of open containers. Inline events go to the
//! innermost container; closing a container turns it into a block, an inline
//! or a list item of its parent. Tight list items carry their text without a
//! paragraph, so text that lands directly in an item (or a quote) is held as
//! a pending paragraph until a block follows or the container closes.

use pulldown_cmark::{Event, Options, Parser, Tag};

use super::{Block, Inline, ListItem};

/// Parses `markdown` (CommonMark, task lists, strikethrough) into blocks.
pub fn parse(markdown: &str) -> Vec<Block> {
    let options = Options::ENABLE_TASKLISTS | Options::ENABLE_STRIKETHROUGH;
    let mut builder = Builder::default();
    for event in Parser::new_ext(markdown, options) {
        builder.event(event);
    }
    builder.finish()
}

/// What an open container becomes when it closes.
#[derive(Debug)]
enum Kind {
    Root,
    Quote,
    List {
        ordered: bool,
    },
    Item {
        checked: Option<bool>,
    },
    Paragraph,
    Heading(u8),
    Strong,
    Emphasis,
    Strike,
    Link(String),
    /// An image: its alt text stays inline.
    Image,
    CodeBlock,
}

/// An open container and what it has collected so far.
#[derive(Debug)]
struct Container {
    kind: Kind,
    blocks: Vec<Block>,
    inlines: Vec<Inline>,
    items: Vec<ListItem>,
    code: String,
}

impl Container {
    fn new(kind: Kind) -> Self {
        Self {
            kind,
            blocks: Vec::new(),
            inlines: Vec::new(),
            items: Vec::new(),
            code: String::new(),
        }
    }

    /// Moves text that arrived outside a paragraph into one.
    fn flush_pending(&mut self) {
        if !self.inlines.is_empty() {
            let content = std::mem::take(&mut self.inlines);
            self.blocks.push(Block::Paragraph { content });
        }
    }
}

/// What a closed container hands its parent.
enum Closed {
    Block(Block),
    Inlines(Vec<Inline>),
    Item(ListItem),
}

struct Builder {
    stack: Vec<Container>,
}

impl Default for Builder {
    fn default() -> Self {
        Self {
            stack: vec![Container::new(Kind::Root)],
        }
    }
}

impl Builder {
    fn event(&mut self, event: Event<'_>) {
        match event {
            Event::Start(tag) => self.stack.push(Container::new(kind_of(tag))),
            Event::End(_) => self.close(),
            Event::Text(text) | Event::Html(text) | Event::InlineHtml(text) => self.text(&text),
            Event::Code(text) => self.inline(Inline::Code {
                text: text.into_string(),
            }),
            Event::SoftBreak => self.text(" "),
            Event::HardBreak => self.inline(Inline::Break),
            Event::Rule => self.block(Block::Rule),
            Event::TaskListMarker(done) => self.check(done),
            _ => {}
        }
    }

    fn finish(mut self) -> Vec<Block> {
        while self.stack.len() > 1 {
            self.close();
        }
        let mut root = self
            .stack
            .pop()
            .unwrap_or_else(|| Container::new(Kind::Root));
        root.flush_pending();
        root.blocks
    }

    fn top(&mut self) -> Option<&mut Container> {
        self.stack.last_mut()
    }

    /// Text joins the previous run when both are plain text.
    fn text(&mut self, text: &str) {
        let Some(top) = self.top() else { return };
        if let Kind::CodeBlock = top.kind {
            top.code.push_str(text);
            return;
        }
        if let Some(Inline::Text { text: last }) = top.inlines.last_mut() {
            last.push_str(text);
            return;
        }
        top.inlines.push(Inline::text(text));
    }

    fn inline(&mut self, inline: Inline) {
        if let Some(top) = self.top() {
            top.inlines.push(inline);
        }
    }

    fn block(&mut self, block: Block) {
        if let Some(top) = self.top() {
            top.flush_pending();
            top.blocks.push(block);
        }
    }

    fn check(&mut self, done: bool) {
        if let Some(Container {
            kind: Kind::Item { checked },
            ..
        }) = self.top()
        {
            *checked = Some(done);
        }
    }

    fn close(&mut self) {
        if self.stack.len() <= 1 {
            return;
        }
        let Some(container) = self.stack.pop() else {
            return;
        };
        match closed(container) {
            Closed::Block(block) => self.block(block),
            Closed::Inlines(inlines) => inlines.into_iter().for_each(|inline| match inline {
                Inline::Text { text } => self.text(&text),
                other => self.inline(other),
            }),
            Closed::Item(item) => {
                if let Some(top) = self.top() {
                    top.items.push(item);
                }
            }
        }
    }
}

fn kind_of(tag: Tag<'_>) -> Kind {
    match tag {
        Tag::BlockQuote(_) => Kind::Quote,
        Tag::List(start) => Kind::List {
            ordered: start.is_some(),
        },
        Tag::Item => Kind::Item { checked: None },
        Tag::Heading { level, .. } => Kind::Heading(level as u8),
        Tag::Strong => Kind::Strong,
        Tag::Emphasis => Kind::Emphasis,
        Tag::Strikethrough => Kind::Strike,
        Tag::Link { dest_url, .. } => Kind::Link(dest_url.into_string()),
        Tag::Image { .. } => Kind::Image,
        Tag::CodeBlock(_) => Kind::CodeBlock,
        _ => Kind::Paragraph,
    }
}

fn closed(mut container: Container) -> Closed {
    let content = std::mem::take(&mut container.inlines);
    match container.kind {
        Kind::Paragraph => Closed::Block(Block::Paragraph { content }),
        Kind::Heading(level) => Closed::Block(Block::Heading { level, content }),
        Kind::CodeBlock => Closed::Block(Block::Code {
            text: container.code.trim_end_matches('\n').to_owned(),
        }),
        Kind::List { ordered } => Closed::Block(Block::List {
            ordered,
            items: container.items,
        }),
        Kind::Strong => Closed::Inlines(vec![Inline::Strong { content }]),
        Kind::Emphasis => Closed::Inlines(vec![Inline::Emphasis { content }]),
        Kind::Strike => Closed::Inlines(vec![Inline::Strike { content }]),
        Kind::Link(href) => Closed::Inlines(vec![Inline::Link { href, content }]),
        Kind::Image => Closed::Inlines(content),
        Kind::Item { checked } => Closed::Item(ListItem {
            checked,
            blocks: blocks_of(container.blocks, content),
        }),
        Kind::Quote | Kind::Root => Closed::Block(Block::Quote {
            blocks: blocks_of(container.blocks, content),
        }),
    }
}

/// `blocks` followed by any pending text as a final paragraph.
fn blocks_of(mut blocks: Vec<Block>, pending: Vec<Inline>) -> Vec<Block> {
    if !pending.is_empty() {
        blocks.push(Block::Paragraph { content: pending });
    }
    blocks
}

#[cfg(test)]
#[path = "parse_tests.rs"]
mod tests;
