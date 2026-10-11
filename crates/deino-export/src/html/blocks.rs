//! [`Block`]s as HTML that survives a paste into Gmail, Google Docs, Notes
//! and Slack: plain tags, a few inline styles, every text node escaped and
//! only web and mail links kept.

use crate::doc::{Block, Inline, ListItem};

/// Inline style of secondary text (`--fg-muted`, light theme).
pub(crate) const MUTED: &str = "color:#65655F";

/// Link schemes a pasted note may keep; anything else becomes plain text.
const SAFE_SCHEMES: &[&str] = &["https://", "http://", "mailto:"];

/// `text` with the five HTML special characters escaped.
pub(crate) fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for char in text.chars() {
        match char {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            other => out.push(other),
        }
    }
    out
}

/// `blocks` as HTML.
pub(crate) fn blocks(blocks: &[Block]) -> String {
    blocks.iter().map(block).collect()
}

fn block(block: &Block) -> String {
    match block {
        Block::Heading { level, content } => {
            let level = (*level).clamp(1, 6);
            format!("<h{level}>{}</h{level}>", inlines(content))
        }
        Block::Paragraph { content } => format!("<p>{}</p>", inlines(content)),
        Block::List { ordered, items } => list(*ordered, items),
        Block::Quote { blocks: quoted } => format!(
            "<blockquote style=\"margin:0 0 0 2px;padding-left:12px;border-left:2px solid #E6E6E1;{MUTED}\">{}</blockquote>",
            blocks(quoted)
        ),
        Block::Code { text } => format!("<pre><code>{}</code></pre>", escape(text)),
        Block::Rule => "<hr>".to_owned(),
    }
}

fn list(ordered: bool, items: &[ListItem]) -> String {
    let tasks = items.iter().any(|item| item.checked.is_some());
    let tag = if ordered { "ol" } else { "ul" };
    let style = if tasks {
        " style=\"list-style:none;padding-left:4px\""
    } else {
        ""
    };
    let body: String = items.iter().map(item).collect();
    format!("<{tag}{style}>{body}</{tag}>")
}

fn item(item: &ListItem) -> String {
    let mark = match item.checked {
        Some(true) => "☑ ",
        Some(false) => "☐ ",
        None => "",
    };
    // A lone paragraph is unwrapped so pasted lists stay tight.
    let body = match item.blocks.as_slice() {
        [Block::Paragraph { content }, rest @ ..] => {
            format!("{}{}", inlines(content), blocks(rest))
        }
        other => blocks(other),
    };
    format!("<li>{mark}{body}</li>")
}

/// `content` as inline HTML.
pub(crate) fn inlines(content: &[Inline]) -> String {
    content.iter().map(inline).collect()
}

fn inline(inline: &Inline) -> String {
    match inline {
        Inline::Text { text } => escape(text),
        Inline::Strong { content } => format!("<strong>{}</strong>", inlines(content)),
        Inline::Emphasis { content } => format!("<em>{}</em>", inlines(content)),
        Inline::Strike { content } => format!("<s>{}</s>", inlines(content)),
        Inline::Code { text } => format!("<code>{}</code>", escape(text)),
        Inline::Link { href, content } if is_safe(href) => {
            format!("<a href=\"{}\">{}</a>", escape(href), inlines(content))
        }
        Inline::Link { content, .. } => inlines(content),
        Inline::Break => "<br>".to_owned(),
    }
}

fn is_safe(href: &str) -> bool {
    let lower = href.trim().to_ascii_lowercase();
    SAFE_SCHEMES.iter().any(|scheme| lower.starts_with(scheme))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::doc::parse;

    #[test]
    fn notes_become_plain_tags() {
        let html = blocks(&parse("## Plan\n\n- [ ] ship **v2**\n- [x] done\n\n1. one"));
        assert_eq!(
            html,
            "<h2>Plan</h2><ul style=\"list-style:none;padding-left:4px\"><li>☐ ship <strong>v2</strong></li><li>☑ done</li></ul><ol><li>one</li></ol>"
        );
    }

    #[test]
    fn text_is_escaped_and_unsafe_links_are_dropped() {
        let html = blocks(&parse(
            "see <img onerror=x> [ok](https://deino.dev?a=1&b=\"2\") [bad](javascript:alert(1))",
        ));
        assert_eq!(
            html,
            "<p>see &lt;img onerror=x&gt; <a href=\"https://deino.dev?a=1&amp;b=&quot;2&quot;\">ok</a> bad</p>"
        );
    }
}
