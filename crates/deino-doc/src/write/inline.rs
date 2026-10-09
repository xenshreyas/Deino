//! Inline content → Markdown, opening and closing marks like a stack.
//!
//! Marks are sorted by [`Mark::rank`], so the marks two neighbouring nodes
//! share form a common prefix: the writer closes what the next node does not
//! have, opens what it adds, and keeps the rest open (`**a *b* c**`).
//!
//! CommonMark only recognises `**`, `*` and `~~` next to non-whitespace
//! (`** a**` is not bold), so whitespace at the edge of those marks moves
//! just outside them, as prosemirror-markdown does. Links and the HTML marks
//! (origin, underline) keep their whitespace exactly.

use super::Flavor;
use super::block::longest_run;
use super::escape::{escape_text, escape_title};
use crate::html;
use crate::{CitationAttrs, Inline, Mark, MentionAttrs};

/// Where the inline content sits.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Context {
    /// A paragraph: line breaks are kept.
    Paragraph,
    /// A heading, which is one line: breaks become spaces.
    Heading,
}

/// Writes `content` as one paragraph's or heading's Markdown.
pub(super) fn inlines(content: &[Inline], flavor: Flavor, place: Context) -> String {
    let mut writer = Writer {
        flavor,
        context: place,
        out: String::new(),
        open: Vec::new(),
        line_start: true,
    };
    for inline in content {
        writer.inline(inline);
    }
    writer.close_to(0);
    writer.out
}

struct Writer {
    flavor: Flavor,
    context: Context,
    out: String,
    /// The marks currently open, outermost first.
    open: Vec<Mark>,
    /// Whether the next character begins a line.
    line_start: bool,
}

impl Writer {
    fn inline(&mut self, inline: &Inline) {
        match inline {
            Inline::Text { text, marks } => self.text(text, marks),
            Inline::HardBreak { marks } => self.line_break(marks),
            Inline::Mention { attrs, marks } => {
                self.open_marks(marks, "");
                let mention = self.mention(attrs);
                self.write(&mention);
            }
            Inline::Citation { attrs, marks } => {
                self.open_marks(marks, "");
                let citation = self.citation(attrs);
                self.write(&citation);
            }
        }
    }

    /// A text run. Whitespace-only text opens no mark (`** **` would be
    /// literal stars); code is one span, whitespace included.
    fn text(&mut self, text: &str, marks: &[Mark]) {
        if marks.contains(&Mark::Code) {
            self.open_marks(marks, "");
            return self.write(&code_span(text));
        }
        let body = text.trim_start();
        let space = text.get(..text.len() - body.len()).unwrap_or_default();
        if body.is_empty() {
            let keep = self.shared_prefix(marks);
            self.close_to(keep);
            return self.write_text(space);
        }
        self.open_marks(marks, space);
        self.write_text(body);
    }

    /// A line break keeps the marks it shares with the text around it and
    /// never opens one.
    fn line_break(&mut self, marks: &[Mark]) {
        let keep = self.shared_prefix(marks);
        self.close_to(keep);
        match (self.context, self.flavor) {
            (Context::Heading, _) => self.out.push(' '),
            (Context::Paragraph, Flavor::Lossless) => self.out.push_str("\\\n"),
            (Context::Paragraph, Flavor::Readable) => self.out.push('\n'),
        }
        self.line_start = self.context == Context::Paragraph;
    }

    /// The marks this flavor writes, without code (written as a span).
    fn visible(&self, marks: &[Mark]) -> Vec<Mark> {
        marks
            .iter()
            .filter(|mark| !matches!(mark, Mark::Code))
            .filter(|mark| self.flavor == Flavor::Lossless || html::open(mark).is_none())
            .cloned()
            .collect()
    }

    /// How many of the open marks `marks` keeps open.
    fn shared_prefix(&self, marks: &[Mark]) -> usize {
        let wanted = self.visible(marks);
        self.open
            .iter()
            .zip(&wanted)
            .take_while(|(open, wanted)| open == wanted)
            .count()
    }

    /// Closes what `marks` drops and opens what it adds. `space` (the text's
    /// leading whitespace) goes before the first emphasis delimiter, or
    /// inside every new mark when none is emphasis.
    fn open_marks(&mut self, marks: &[Mark], space: &str) {
        let keep = self.shared_prefix(marks);
        self.close_to(keep);
        let mut space = Some(space);
        for mark in self.visible(marks).into_iter().skip(keep) {
            if is_emphasis(&mark)
                && let Some(space) = space.take()
            {
                self.write_text(space);
            }
            self.write(&opening(&mark));
            self.open.push(mark);
        }
        if let Some(space) = space {
            self.write_text(space);
        }
    }

    /// Closes marks until `len` stay open. Trailing spaces move after the
    /// last emphasis closer, and stay inside links and HTML marks.
    fn close_to(&mut self, len: usize) {
        if self.open.len() <= len {
            return;
        }
        let closing: Vec<Mark> = self.open.drain(len..).rev().collect();
        let last_emphasis = closing.iter().rposition(is_emphasis);
        let spaces = if last_emphasis.is_some() {
            let kept = self.out.trim_end_matches([' ', '\t']).len();
            self.out.split_off(kept)
        } else {
            String::new()
        };
        for (index, mark) in closing.iter().enumerate() {
            self.out.push_str(&closing_delimiter(mark));
            if Some(index) == last_emphasis {
                self.out.push_str(&spaces);
            }
        }
    }

    fn write_text(&mut self, text: &str) {
        let escaped = escape_text(text, self.line_start);
        self.write(&escaped);
    }

    fn write(&mut self, markdown: &str) {
        if !markdown.is_empty() {
            self.out.push_str(markdown);
            self.line_start = false;
        }
    }

    fn mention(&self, attrs: &MentionAttrs) -> String {
        match self.flavor {
            Flavor::Lossless => html::mention_tag(attrs),
            Flavor::Readable => format!("@{}", escape_text(&attrs.label, false)),
        }
    }

    fn citation(&self, attrs: &CitationAttrs) -> String {
        match self.flavor {
            Flavor::Lossless => html::citation_tag(attrs),
            Flavor::Readable => format!("[{}]", timestamp(attrs.start_ms.get())),
        }
    }
}

/// Marks written with `*`/`~` delimiters, which must touch non-whitespace.
fn is_emphasis(mark: &Mark) -> bool {
    matches!(mark, Mark::Bold | Mark::Italic | Mark::Strike)
}

fn opening(mark: &Mark) -> String {
    match mark {
        Mark::Link { .. } => "[".to_owned(),
        Mark::Bold => "**".to_owned(),
        Mark::Italic => "*".to_owned(),
        Mark::Strike => "~~".to_owned(),
        other => html::open(other).unwrap_or_default(),
    }
}

fn closing_delimiter(mark: &Mark) -> String {
    match mark {
        Mark::Link { attrs } => match &attrs.title {
            Some(title) => format!(
                "]({} \"{}\")",
                link_target(&attrs.href),
                escape_title(title)
            ),
            None => format!("]({})", link_target(&attrs.href)),
        },
        Mark::Bold => "**".to_owned(),
        Mark::Italic => "*".to_owned(),
        Mark::Strike => "~~".to_owned(),
        other => html::close(other).unwrap_or_default().to_owned(),
    }
}

/// A link destination; one with spaces, parentheses or angle brackets goes in `<…>`.
fn link_target(href: &str) -> String {
    if href.contains([' ', '(', ')', '<', '>']) {
        format!("<{}>", href.replace('<', "%3C").replace('>', "%3E"))
    } else {
        href.to_owned()
    }
}

/// A code span whose backtick fence is longer than any run inside it.
fn code_span(code: &str) -> String {
    let fence = "`".repeat(longest_run(code, '`') + 1);
    let padded = code.starts_with('`')
        || code.ends_with('`')
        || (code.starts_with(' ') && code.ends_with(' ') && !code.trim().is_empty());
    if padded {
        format!("{fence} {code} {fence}")
    } else {
        format!("{fence}{code}{fence}")
    }
}

/// `m:ss`, or `h:mm:ss` from an hour on.
fn timestamp(ms: u32) -> String {
    let seconds = ms / 1000;
    let (hours, minutes, seconds) = (seconds / 3600, seconds / 60 % 60, seconds % 60);
    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes}:{seconds:02}")
    }
}
