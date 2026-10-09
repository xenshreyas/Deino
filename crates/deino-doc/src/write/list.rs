//! Lists → Markdown: markers, item indentation and tight nesting.

use super::Flavor;
use super::block::render;
use crate::Block;

/// How a list is marked. CommonMark joins neighbouring lists that use the
/// same marker, so a list right after one of its kind takes the alternate.
#[derive(Clone, Copy)]
pub(super) struct Markers {
    bullet: &'static str,
    delimiter: char,
}

const PRIMARY: Markers = Markers {
    bullet: "- ",
    delimiter: '.',
};

const ALTERNATE: Markers = Markers {
    bullet: "* ",
    delimiter: ')',
};

/// Lists that would merge if written one after another with one marker.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Family {
    Bulleted,
    Numbered,
}

fn family(block: &Block) -> Option<Family> {
    match block {
        Block::BulletList { .. } | Block::TaskList { .. } => Some(Family::Bulleted),
        Block::OrderedList { .. } => Some(Family::Numbered),
        _ => None,
    }
}

/// Each block's markers: a list alternates with an unalternated list of its
/// family right before it.
pub(super) fn markers(content: &[Block]) -> Vec<Markers> {
    let mut markers = Vec::with_capacity(content.len());
    let mut previous: Option<(Family, bool)> = None;
    for block in content {
        let family = family(block);
        let alternate = matches!(previous, Some((before, false)) if Some(before) == family);
        markers.push(if alternate { ALTERNATE } else { PRIMARY });
        previous = family.map(|family| (family, alternate));
    }
    markers
}

/// A list block: bulleted, numbered from its start, or a checklist.
pub(super) fn list(block: &Block, flavor: Flavor, markers: Markers) -> String {
    let items: Vec<(String, &str, &[Block])> = match block {
        Block::BulletList { content } => content
            .iter()
            .map(|item| (markers.bullet.to_owned(), "", &item.content[..]))
            .collect(),
        Block::OrderedList { attrs, content } => content
            .iter()
            .enumerate()
            .map(|(index, item)| {
                let number =
                    u32::try_from(index).map_or(u32::MAX, |n| attrs.start.saturating_add(n));
                (
                    format!("{number}{} ", markers.delimiter),
                    "",
                    &item.content[..],
                )
            })
            .collect(),
        Block::TaskList { content } => content
            .iter()
            .map(|item| {
                let checkbox = if item.attrs.checked { "[x] " } else { "[ ] " };
                (markers.bullet.to_owned(), checkbox, &item.content[..])
            })
            .collect(),
        _ => Vec::new(),
    };
    items
        .into_iter()
        .map(|(marker, prefix, content)| item(&marker, prefix, content, flavor))
        .collect::<Vec<_>>()
        .join("\n")
}

/// One item: the marker before its first line, and every later line
/// indented to the marker's width so it stays inside the item.
fn item(marker: &str, prefix: &str, content: &[Block], flavor: Flavor) -> String {
    let body = item_body(content, flavor);
    let indent = " ".repeat(marker.len());
    let mut lines = body.split('\n');
    let first = lines.next().unwrap_or_default();
    let mut markdown = format!("{marker}{prefix}{first}");
    for line in lines {
        markdown.push('\n');
        if !line.is_empty() {
            markdown.push_str(&indent);
            markdown.push_str(line);
        }
    }
    markdown
}

/// An item's blocks: a nested list follows its paragraph directly (keeping
/// the list tight), anything else after a blank line.
fn item_body(content: &[Block], flavor: Flavor) -> String {
    let mut body = String::new();
    for (index, (block, markers)) in content.iter().zip(markers(content)).enumerate() {
        if index > 0 {
            body.push_str(if family(block).is_some() {
                "\n"
            } else {
                "\n\n"
            });
        }
        body.push_str(&render(block, flavor, markers));
    }
    body
}
