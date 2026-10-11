//! [`Block`]s back to Markdown, with every piece of meeting text escaped so
//! it reads as written (and never becomes an Obsidian tag or link).

use crate::doc::{Block, Inline, ListItem};

/// Characters that mean something inline in CommonMark or Obsidian.
const SPECIAL: &[char] = &['\\', '`', '*', '_', '[', ']', '<', '>', '~', '#', '|'];

/// `text` with Markdown syntax escaped.
pub(crate) fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for char in text.chars() {
        if SPECIAL.contains(&char) {
            out.push('\\');
        }
        out.push(char);
    }
    guard_line_start(&out)
}

/// Escapes what would start a block at the beginning of a line: `+`, `-`,
/// `=` and `1.`-style markers (`#` and `>` are already escaped).
fn guard_line_start(text: &str) -> String {
    let first_word: String = text.chars().take_while(char::is_ascii_digit).collect();
    let rest = text.strip_prefix(first_word.as_str()).unwrap_or(text);
    if !first_word.is_empty() && (rest.starts_with(". ") || rest.starts_with(") ")) {
        return format!("{first_word}\\{rest}");
    }
    if text.starts_with(['+', '-', '=']) {
        return format!("\\{text}");
    }
    text.to_owned()
}

/// `blocks` as Markdown, separated by blank lines.
pub(crate) fn blocks(blocks: &[Block]) -> String {
    blocks.iter().map(block).collect::<Vec<_>>().join("\n\n")
}

fn block(block: &Block) -> String {
    match block {
        Block::Heading { level, content } => {
            format!("{} {}", "#".repeat(usize::from(*level)), inlines(content))
        }
        Block::Paragraph { content } => inlines(content),
        Block::List { ordered, items } => list(*ordered, items),
        Block::Quote { blocks: quoted } => prefix_lines(&blocks(quoted), "> ", "> "),
        Block::Code { text } => fenced(text),
        Block::Rule => "---".to_owned(),
    }
}

fn list(ordered: bool, items: &[ListItem]) -> String {
    items
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let marker = if ordered {
                format!("{}. ", index + 1)
            } else {
                "- ".to_owned()
            };
            let task = match item.checked {
                Some(true) => "[x] ",
                Some(false) => "[ ] ",
                None => "",
            };
            let indent = " ".repeat(marker.len());
            let body = item_body(&item.blocks);
            prefix_lines(&body, &format!("{marker}{task}"), &indent)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// An item's blocks: a nested list follows its paragraph without a gap.
fn item_body(item: &[Block]) -> String {
    item.iter().map(block).collect::<Vec<_>>().join("\n")
}

/// `text` with `first` before its first line and `rest` before the others
/// (blank lines stay blank).
fn prefix_lines(text: &str, first: &str, rest: &str) -> String {
    text.lines()
        .enumerate()
        .map(|(index, line)| match (index, line.is_empty()) {
            (0, _) => format!("{first}{line}"),
            (_, true) => rest.trim_end().to_owned(),
            (_, false) => format!("{rest}{line}"),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// A fence longer than any backtick run in the code.
fn fenced(code: &str) -> String {
    let fence = "`".repeat(longest_backtick_run(code).max(2) + 1);
    format!("{fence}\n{code}\n{fence}")
}

fn longest_backtick_run(text: &str) -> usize {
    text.split(|char| char != '`')
        .map(str::len)
        .max()
        .unwrap_or(0)
}

/// `content` as inline Markdown.
pub(crate) fn inlines(content: &[Inline]) -> String {
    content.iter().map(inline).collect()
}

fn inline(inline: &Inline) -> String {
    match inline {
        Inline::Text { text } => escape(text),
        Inline::Strong { content } => format!("**{}**", inlines(content)),
        Inline::Emphasis { content } => format!("*{}*", inlines(content)),
        Inline::Strike { content } => format!("~~{}~~", inlines(content)),
        Inline::Code { text } => code_span(text),
        Inline::Link { href, content } => {
            format!("[{}](<{}>)", inlines(content), href.replace('>', "%3E"))
        }
        Inline::Break => "\\\n".to_owned(),
    }
}

fn code_span(text: &str) -> String {
    let ticks = "`".repeat(longest_backtick_run(text) + 1);
    let pad = if text.starts_with('`') || text.ends_with('`') {
        " "
    } else {
        ""
    };
    format!("{ticks}{pad}{text}{pad}{ticks}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::doc::parse;

    fn round_trip(markdown: &str) -> String {
        blocks(&parse(markdown))
    }

    #[test]
    fn notes_survive_a_round_trip() {
        let notes = "## Overview\n\n- [ ] Ship **v2**\n- [x] Fix *auth*\n  - nested item\n\n1. first\n2. second\n\n> quoted\n\n---";
        assert_eq!(round_trip(notes), notes);
    }

    #[test]
    fn meeting_text_never_turns_into_markup() {
        assert_eq!(
            escape("#launch [[Maya]] *now* <b>"),
            "\\#launch \\[\\[Maya\\]\\] \\*now\\* \\<b\\>"
        );
        assert_eq!(escape("- not a bullet"), "\\- not a bullet");
        assert_eq!(escape("2026. A year"), "2026\\. A year");
    }

    #[test]
    fn code_keeps_its_backticks() {
        assert_eq!(round_trip("use `a`` b`"), "use ```a`` b```");
        assert_eq!(fenced("x ``` y"), "````\nx ``` y\n````");
    }
}
