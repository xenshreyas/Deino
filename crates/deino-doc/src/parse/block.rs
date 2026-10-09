//! Block-level mdast nodes → [`Block`]s.

use markdown::mdast::{self, Node};

use super::inline::{Segment, segments};
use crate::{
    Block, CodeBlockAttrs, HeadingAttrs, ImageAttrs, Inline, ListItem, OrderedListAttrs, TaskItem,
    TaskItemAttrs,
};

/// The blocks for a run of mdast flow nodes.
pub(super) fn blocks(nodes: &[Node]) -> Vec<Block> {
    let mut out = Vec::new();
    for node in nodes {
        push_block(node, &mut out);
    }
    out
}

/// Appends the blocks one mdast node becomes (none, one, or several when
/// images are lifted out of a paragraph).
fn push_block(node: &Node, out: &mut Vec<Block>) {
    match node {
        Node::Paragraph(paragraph) => push_text_block(&paragraph.children, Block::paragraph, out),
        Node::Heading(heading) => push_heading(heading, out),
        Node::List(list) => out.push(list_block(list)),
        Node::Code(code) => out.push(code_block(&code.value, code.lang.clone())),
        Node::Math(math) => out.push(code_block(&math.value, Some("math".into()))),
        Node::Blockquote(quote) => out.push(Block::Blockquote {
            content: non_empty(blocks(&quote.children)),
        }),
        Node::ThematicBreak(_) => out.push(Block::HorizontalRule),
        Node::Html(_) => push_text_block(std::slice::from_ref(node), Block::paragraph, out),
        Node::Table(table) => push_table(table, out),
        Node::Definition(_) | Node::Yaml(_) | Node::Toml(_) => {}
        other => push_plain_text(other, out),
    }
}

fn push_heading(heading: &mdast::Heading, out: &mut Vec<Block>) {
    let attrs = HeadingAttrs {
        level: heading.depth,
    };
    push_text_block(
        &heading.children,
        |content| Block::Heading { attrs, content },
        out,
    );
}

/// A paragraph-like block from phrasing content. Images become their own
/// blocks after the text before them; blank stretches between images vanish.
fn push_text_block(nodes: &[Node], make: impl Fn(Vec<Inline>) -> Block, out: &mut Vec<Block>) {
    let mut pending = Vec::new();
    let mut lifted_images = false;
    for segment in segments(nodes) {
        match segment {
            Segment::Inline(inline) => pending.push(inline),
            Segment::Image(attrs) => {
                let before = trim_breaks(std::mem::take(&mut pending));
                if !before.iter().all(Inline::is_blank) {
                    out.push(make(before));
                }
                out.push(Block::Image { attrs });
                lifted_images = true;
            }
        }
    }
    let rest = trim_breaks(pending);
    if !lifted_images || !rest.iter().all(Inline::is_blank) {
        out.push(make(rest));
    }
}

/// Drops line breaks at either end, which images or block edges left behind.
fn trim_breaks(mut content: Vec<Inline>) -> Vec<Inline> {
    while content.last().is_some_and(is_break) {
        content.pop();
    }
    let leading = content.iter().take_while(|inline| is_break(inline)).count();
    content.drain(..leading);
    content
}

fn is_break(inline: &Inline) -> bool {
    matches!(inline, Inline::HardBreak { .. })
}

fn list_block(list: &mdast::List) -> Block {
    let items = list.children.iter().filter_map(|child| match child {
        Node::ListItem(item) => Some(item),
        _ => None,
    });
    if list.children.iter().any(is_task_item) {
        let content = items.map(task_item).collect();
        return Block::TaskList { content };
    }
    let content = items.map(|item| ListItem {
        content: item_blocks(item),
    });
    if list.ordered {
        let attrs = OrderedListAttrs {
            start: list.start.unwrap_or(1),
        };
        return Block::OrderedList {
            attrs,
            content: content.collect(),
        };
    }
    Block::BulletList {
        content: content.collect(),
    }
}

fn is_task_item(node: &Node) -> bool {
    matches!(node, Node::ListItem(item) if item.checked.is_some())
}

fn task_item(item: &mdast::ListItem) -> TaskItem {
    TaskItem {
        attrs: TaskItemAttrs {
            checked: item.checked.unwrap_or(false),
        },
        content: item_blocks(item),
    }
}

/// A list item's blocks; TipTap requires them to start with a paragraph.
fn item_blocks(item: &mdast::ListItem) -> Vec<Block> {
    let mut content = blocks(&item.children);
    if !matches!(content.first(), Some(Block::Paragraph { .. })) {
        content.insert(0, Block::paragraph(vec![]));
    }
    content
}

fn code_block(value: &str, language: Option<String>) -> Block {
    let content = if value.is_empty() {
        vec![]
    } else {
        vec![Inline::text(value, vec![])]
    };
    Block::CodeBlock {
        attrs: CodeBlockAttrs { language },
        content,
    }
}

/// A table has no node in the editor: each row becomes a paragraph with its
/// cells separated by ` | `, so no text is lost.
fn push_table(table: &mdast::Table, out: &mut Vec<Block>) {
    for row in &table.children {
        let cells = row.children().map(Vec::as_slice).unwrap_or_default();
        let mut content = Vec::new();
        for (index, cell) in cells.iter().enumerate() {
            if index > 0 {
                content.push(Inline::text(" | ", vec![]));
            }
            let phrasing = cell.children().map(Vec::as_slice).unwrap_or_default();
            content.extend(
                segments(phrasing)
                    .into_iter()
                    .filter_map(Segment::into_inline),
            );
        }
        if !content.is_empty() {
            out.push(Block::paragraph(content));
        }
    }
}

/// Anything else keeps its text as a paragraph.
fn push_plain_text(node: &Node, out: &mut Vec<Block>) {
    let text = node.to_string();
    if !text.trim().is_empty() {
        out.push(Block::paragraph(vec![Inline::text(text, vec![])]));
    }
}

/// A container must hold at least one block.
fn non_empty(content: Vec<Block>) -> Vec<Block> {
    if content.is_empty() {
        vec![Block::paragraph(vec![])]
    } else {
        content
    }
}

/// An image block's attributes from an mdast image.
pub(super) fn image_attrs(image: &mdast::Image) -> ImageAttrs {
    ImageAttrs {
        src: image.url.clone(),
        alt: (!image.alt.is_empty()).then(|| image.alt.clone()),
        title: image.title.clone(),
    }
}
