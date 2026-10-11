//! [`Block`]s as plain text: bullets as `•`, tasks as `[ ]`/`[x]`, nested
//! lists indented two spaces, quotes prefixed with `>`.

use crate::doc::{Block, ListItem, plain};

/// How far each nested list level is indented.
const INDENT: &str = "  ";

/// `blocks` as plain text, separated by blank lines.
pub(crate) fn blocks(blocks: &[Block]) -> String {
    blocks.iter().map(block).collect::<Vec<_>>().join("\n\n")
}

fn block(block: &Block) -> String {
    match block {
        Block::Heading { content, .. } | Block::Paragraph { content } => plain(content),
        Block::List { ordered, items } => list(*ordered, items),
        Block::Quote { blocks: quoted } => blocks(quoted)
            .lines()
            .map(|line| format!("> {line}").trim_end().to_owned())
            .collect::<Vec<_>>()
            .join("\n"),
        Block::Code { text } => text
            .lines()
            .map(|line| format!("    {line}"))
            .collect::<Vec<_>>()
            .join("\n"),
        Block::Rule => "———".to_owned(),
    }
}

fn list(ordered: bool, items: &[ListItem]) -> String {
    items
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let marker = match (item.checked, ordered) {
                (Some(true), _) => "[x] ".to_owned(),
                (Some(false), _) => "[ ] ".to_owned(),
                (None, true) => format!("{}. ", index + 1),
                (None, false) => "• ".to_owned(),
            };
            let body = item.blocks.iter().map(block).collect::<Vec<_>>().join("\n");
            indent_after_first(&format!("{marker}{body}"))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn indent_after_first(text: &str) -> String {
    text.lines()
        .enumerate()
        .map(|(index, line)| {
            if index == 0 {
                line.to_owned()
            } else {
                format!("{INDENT}{line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::doc::parse;

    #[test]
    fn notes_read_cleanly_without_markup() {
        let text = blocks(&parse(
            "## Overview\n\n- **Ship** v2\n  - after QA\n- [x] done\n\n1. one\n\n> quoted",
        ));
        assert_eq!(
            text,
            "Overview\n\n• Ship v2\n  • after QA\n[x] done\n\n1. one\n\n> quoted"
        );
    }
}
