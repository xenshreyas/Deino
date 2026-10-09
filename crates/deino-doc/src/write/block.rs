//! Blocks → Markdown. Containers render their children first and then
//! prefix the lines (`- `, `> `, indentation), so nesting composes.

use super::Flavor;
use super::escape::{escape_link_text, escape_title};
use super::inline::{Context, inlines};
use super::list::{Markers, list, markers};
use crate::{Block, CodeBlockAttrs, ImageAttrs, Inline};

/// The blocks, separated by blank lines; blocks that render empty are skipped.
pub(super) fn blocks(content: &[Block], flavor: Flavor) -> String {
    content
        .iter()
        .zip(markers(content))
        .map(|(block, markers)| render(block, flavor, markers))
        .filter(|markdown| !markdown.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// One block; `markers` matter only for lists.
pub(super) fn render(block: &Block, flavor: Flavor, markers: Markers) -> String {
    match block {
        Block::Paragraph { content } => inlines(content, flavor, Context::Paragraph),
        Block::Heading { attrs, content } => {
            let text = inlines(content, flavor, Context::Heading);
            format!("{} {text}", "#".repeat(usize::from(attrs.level)))
        }
        Block::BulletList { .. } | Block::OrderedList { .. } | Block::TaskList { .. } => {
            list(block, flavor, markers)
        }
        Block::Blockquote { content } => prefix_lines(&blocks(content, flavor), "> ", ">"),
        Block::CodeBlock { attrs, content } => code_block(attrs, content),
        Block::HorizontalRule => "---".to_owned(),
        Block::Image { attrs } => image(attrs),
    }
}

/// `prefix` before every line, `empty` on blank ones.
fn prefix_lines(text: &str, prefix: &str, empty: &str) -> String {
    text.split('\n')
        .map(|line| {
            if line.is_empty() {
                empty.to_owned()
            } else {
                format!("{prefix}{line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// A fenced code block whose fence is longer than any backtick run inside.
fn code_block(attrs: &CodeBlockAttrs, content: &[Inline]) -> String {
    let code: String = content
        .iter()
        .filter_map(|inline| match inline {
            Inline::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    let fence = "`".repeat(longest_run(&code, '`').max(2) + 1);
    let language = attrs.language.as_deref().unwrap_or_default();
    if code.is_empty() {
        return format!("{fence}{language}\n{fence}");
    }
    let newline = if code.ends_with('\n') { "" } else { "\n" };
    format!("{fence}{language}\n{code}{newline}{fence}")
}

/// The longest run of `needle` in `text`.
pub(super) fn longest_run(text: &str, needle: char) -> usize {
    let mut longest = 0;
    let mut current = 0;
    for character in text.chars() {
        current = if character == needle { current + 1 } else { 0 };
        longest = longest.max(current);
    }
    longest
}

/// `![alt](src "title")`; a source with spaces or parentheses goes in `<…>`.
fn image(attrs: &ImageAttrs) -> String {
    let alt = escape_link_text(attrs.alt.as_deref().unwrap_or_default());
    let src = if attrs.src.contains([' ', '(', ')']) {
        format!("<{}>", attrs.src.replace('<', "%3C").replace('>', "%3E"))
    } else {
        attrs.src.clone()
    };
    match &attrs.title {
        Some(title) => format!("![{alt}]({src} \"{}\")", escape_title(title)),
        None => format!("![{alt}]({src})"),
    }
}
