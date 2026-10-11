//! File and folder names from the user's templates (EXP-02): `{date}
//! {title}.md` for notes, `{yyyy}/{mm}` for the folders they go in.
//!
//! A template is parsed once ([`NameTemplate::parse`] rejects unknown
//! placeholders, so a typo is caught in Settings, not at export time), then
//! expanded per meeting. Every expanded path component goes through
//! [`sanitize`]: no separators, nothing a file system or Obsidian link
//! rejects, no leading dots, at most [`MAX_COMPONENT_CHARS`] characters, so a
//! meeting title can never escape the export folder or fail to save.

use crate::MeetingExport;

/// The longest path component a template expands to, in characters.
pub const MAX_COMPONENT_CHARS: usize = 120;
/// The most bytes a component may take (APFS allows 255; leave room for a
/// suffix such as ` (2)` and the extension).
const MAX_COMPONENT_BYTES: usize = 200;
/// What an empty component becomes.
const UNTITLED: &str = "Untitled";
/// Characters that separate paths or that macOS, Windows or Obsidian reject.
const REPLACED: &[char] = &['/', '\\', ':'];
/// Characters dropped from names: wildcards, quotes and Obsidian link syntax.
const DROPPED: &[char] = &['*', '?', '"', '<', '>', '|', '#', '^', '[', ']'];
/// The note extension templates may end with.
const MARKDOWN_EXTENSION: &str = ".md";

/// A placeholder a template can use.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Field {
    /// `{date}`: 2026-10-08.
    Date,
    /// `{time}`: 15-31.
    Time,
    /// `{title}`: the meeting's title.
    Title,
    /// `{yyyy}`: 2026.
    Year,
    /// `{mm}`: 10.
    Month,
    /// `{dd}`: 08.
    Day,
}

impl Field {
    fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "date" => Self::Date,
            "time" => Self::Time,
            "title" => Self::Title,
            "yyyy" => Self::Year,
            "mm" => Self::Month,
            "dd" => Self::Day,
            _ => return None,
        })
    }

    fn expand(self, export: &MeetingExport) -> String {
        let format = match self {
            Self::Title => return export.title.clone(),
            Self::Date => "%Y-%m-%d",
            Self::Time => "%H-%M",
            Self::Year => "%Y",
            Self::Month => "%m",
            Self::Day => "%d",
        };
        export.started_at.format(format).to_string()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Part {
    Literal(String),
    Field(Field),
}

/// Why a template was rejected. Messages quote only the template, which the
/// user typed, never meeting content.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum TemplateError {
    /// The template expands to nothing.
    #[error("the name template is empty")]
    Empty,
    /// A `{` without its `}`.
    #[error("the name template has a {{ without a closing }}")]
    Unclosed,
    /// A placeholder the template language does not have.
    #[error(
        "{{{0}}} is not a placeholder; use {{date}}, {{time}}, {{title}}, {{yyyy}}, {{mm}} or {{dd}}"
    )]
    UnknownField(String),
}

/// A parsed name template.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NameTemplate {
    parts: Vec<Part>,
}

impl NameTemplate {
    /// Parses `template`; a trailing `.md` is allowed and ignored.
    pub fn parse(template: &str) -> Result<Self, TemplateError> {
        let template = strip_extension(template.trim());
        if template.trim().is_empty() {
            return Err(TemplateError::Empty);
        }
        let mut parts = Vec::new();
        let mut rest = template;
        while let Some(open) = rest.find('{') {
            let (literal, after) = rest.split_at(open);
            let close = after.find('}').ok_or(TemplateError::Unclosed)?;
            let name = after.get(1..close).unwrap_or_default();
            let field =
                Field::parse(name).ok_or_else(|| TemplateError::UnknownField(name.to_owned()))?;
            push_literal(&mut parts, literal);
            parts.push(Part::Field(field));
            rest = after.get(close + 1..).unwrap_or_default();
        }
        push_literal(&mut parts, rest);
        Ok(Self { parts })
    }

    /// The template filled in. A value's own slashes become `-`, so only the
    /// template's literal slashes can make folders.
    fn expand(&self, export: &MeetingExport) -> String {
        self.parts
            .iter()
            .map(|part| match part {
                Part::Literal(text) => text.clone(),
                Part::Field(field) => field.expand(export).replace(['/', '\\'], "-"),
            })
            .collect()
    }

    /// The note's file name: the expansion as one component, plus `.md`.
    pub fn file_name(&self, export: &MeetingExport) -> String {
        format!("{}{MARKDOWN_EXTENSION}", self.stem(export))
    }

    /// The note's name without its extension (what wiki links point at).
    pub fn stem(&self, export: &MeetingExport) -> String {
        sanitize(&self.expand(export))
    }

    /// The folders the note goes in: the expansion split at `/`, each
    /// component sanitized, empty ones dropped.
    pub fn folders(&self, export: &MeetingExport) -> Vec<String> {
        self.expand(export)
            .split('/')
            .filter(|component| !component.trim().is_empty())
            .map(sanitize)
            .collect()
    }
}

fn strip_extension(template: &str) -> &str {
    let split = template.len().saturating_sub(MARKDOWN_EXTENSION.len());
    match template.get(split..) {
        Some(tail) if tail.eq_ignore_ascii_case(MARKDOWN_EXTENSION) => {
            template.get(..split).unwrap_or(template)
        }
        _ => template,
    }
}

fn push_literal(parts: &mut Vec<Part>, literal: &str) {
    if !literal.is_empty() {
        parts.push(Part::Literal(literal.to_owned()));
    }
}

/// `raw` as one safe path component (see the module docs).
pub fn sanitize(raw: &str) -> String {
    // "Interview: Jordan" reads best as "Interview - Jordan".
    let replaced: String = raw
        .replace(": ", " - ")
        .chars()
        .filter(|char| !DROPPED.contains(char))
        .map(|char| {
            if REPLACED.contains(&char) || char.is_control() {
                '-'
            } else {
                char
            }
        })
        .collect();
    let collapsed = replaced.split_whitespace().collect::<Vec<_>>().join(" ");
    let short = truncate(&collapsed);
    let trimmed = short.trim_matches(|char: char| char == '.' || char.is_whitespace());
    if trimmed.is_empty() {
        UNTITLED.to_owned()
    } else {
        trimmed.to_owned()
    }
}

/// At most [`MAX_COMPONENT_CHARS`] characters and [`MAX_COMPONENT_BYTES`] bytes.
fn truncate(text: &str) -> String {
    let mut out = String::new();
    for char in text.chars().take(MAX_COMPONENT_CHARS) {
        if out.len() + char.len_utf8() > MAX_COMPONENT_BYTES {
            break;
        }
        out.push(char);
    }
    out
}

#[cfg(test)]
#[path = "naming_tests.rs"]
mod tests;
