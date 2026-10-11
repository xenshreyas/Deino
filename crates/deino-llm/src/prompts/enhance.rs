//! `notes.enhance` (NOT-02): the prompts that turn a transcript and the
//! user's rough notes into their meeting notes, and the `title` prompt that
//! names a meeting from those notes.
//!
//! A meeting that fits one call uses [`EnhanceSystem`] + [`EnhanceUser`]. A
//! long one is noted in parts ([`EnhancePartSystem`] + [`EnhancePartUser`],
//! map) whose notes are merged ([`EnhanceSystem`] + [`EnhanceMergeUser`],
//! reduce). Every input is rendered to text by `deino_core::enhance`; these
//! structs only place it.

use askama::Template;

/// `notes.enhance`: the rules of the notes (one pass, and the merge).
#[derive(Template)]
#[template(path = "notes_enhance.system.md.jinja", escape = "none")]
pub struct EnhanceSystem;

/// `notes.enhance`, one pass over the whole meeting.
#[derive(Template)]
#[template(path = "notes_enhance.user.md.jinja", escape = "none")]
pub struct EnhanceUser<'a> {
    /// Title, date, attendees and description, one per line.
    pub meeting: &'a str,
    /// The template's name, context and numbered sections.
    pub template: &'a str,
    /// The previous meeting of the series and the start of its notes.
    pub previous: Option<&'a str>,
    /// Names and terms to spell as given, comma-separated.
    pub vocabulary: Option<&'a str>,
    /// The user's own notes, as Markdown.
    pub notes: Option<&'a str>,
    /// One `[s<n> mm:ss] Speaker: text` line per segment.
    pub transcript: &'a str,
}

/// `notes.enhance`, map step: the rules of one part's notes.
#[derive(Template)]
#[template(path = "notes_enhance_part.system.md.jinja", escape = "none")]
pub struct EnhancePartSystem;

/// `notes.enhance`, map step: one part of a long meeting.
#[derive(Template)]
#[template(path = "notes_enhance_part.user.md.jinja", escape = "none")]
pub struct EnhancePartUser<'a> {
    /// Title, date, attendees and description, one per line.
    pub meeting: &'a str,
    /// The template's name, context and numbered sections.
    pub template: &'a str,
    /// The user's own notes, as Markdown.
    pub notes: Option<&'a str>,
    /// This part's number, from 1.
    pub part: usize,
    /// How many parts there are.
    pub parts: usize,
    /// The part's time span, `20:00–40:00`.
    pub span: &'a str,
    /// The part's transcript lines.
    pub transcript: &'a str,
}

/// `notes.enhance`, reduce step: the parts' notes to merge.
#[derive(Template)]
#[template(path = "notes_enhance_merge.user.md.jinja", escape = "none")]
pub struct EnhanceMergeUser<'a> {
    /// Title, date, attendees and description, one per line.
    pub meeting: &'a str,
    /// The template's name, context and numbered sections.
    pub template: &'a str,
    /// The previous meeting of the series and the start of its notes.
    pub previous: Option<&'a str>,
    /// Names and terms to spell as given, comma-separated.
    pub vocabulary: Option<&'a str>,
    /// The user's own notes, as Markdown.
    pub notes: Option<&'a str>,
    /// Each part's notes, every line ending with its references.
    pub partials: &'a str,
}

/// `title`: a meeting's notes to title (NOT-02's `auto_title` stage).
#[derive(Template)]
#[template(path = "title_notes.user.md.jinja", escape = "none")]
pub struct TitleNotesUser<'a> {
    /// The enhanced notes, as Readable Markdown.
    pub notes: &'a str,
}

#[cfg(test)]
mod enhance_tests;
