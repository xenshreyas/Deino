//! What to export and how: the format, the sections and the link style.

use serde::{Deserialize, Serialize};

/// A file format a meeting can be exported in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum ExportFormat {
    /// Markdown with YAML frontmatter.
    Markdown,
    /// Plain text.
    Text,
    /// The whole meeting as JSON.
    Json,
    /// The transcript as SubRip subtitles.
    Srt,
    /// The transcript as WebVTT subtitles.
    Vtt,
    /// A Word document.
    Docx,
    /// A PDF document.
    Pdf,
}

impl ExportFormat {
    /// Every format, in the order the export dialog lists them.
    pub const ALL: [Self; 7] = [
        Self::Markdown,
        Self::Pdf,
        Self::Docx,
        Self::Text,
        Self::Json,
        Self::Srt,
        Self::Vtt,
    ];

    /// The file extension, without the dot.
    pub const fn extension(self) -> &'static str {
        match self {
            Self::Markdown => "md",
            Self::Text => "txt",
            Self::Json => "json",
            Self::Srt => "srt",
            Self::Vtt => "vtt",
            Self::Docx => "docx",
            Self::Pdf => "pdf",
        }
    }

    /// The media type of the rendered file.
    pub const fn mime(self) -> &'static str {
        match self {
            Self::Markdown => "text/markdown; charset=utf-8",
            Self::Text => "text/plain; charset=utf-8",
            Self::Json => "application/json",
            Self::Srt => "application/x-subrip",
            Self::Vtt => "text/vtt; charset=utf-8",
            Self::Docx => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            Self::Pdf => "application/pdf",
        }
    }

    /// The name of the format's file type, for the save dialog's filter.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Markdown => "Markdown",
            Self::Text => "Plain text",
            Self::Json => "JSON",
            Self::Srt => "SubRip subtitles",
            Self::Vtt => "WebVTT subtitles",
            Self::Docx => "Word document",
            Self::Pdf => "PDF document",
        }
    }

    /// False for formats with a fixed content: JSON is always the whole
    /// meeting, and subtitles are always the transcript.
    pub const fn honours_sections(self) -> bool {
        !matches!(self, Self::Json | Self::Srt | Self::Vtt)
    }

    /// True when the rendered bytes are UTF-8 text.
    pub const fn is_text(self) -> bool {
        !matches!(self, Self::Docx | Self::Pdf)
    }
}

/// A part of a meeting an export can include.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Section {
    /// The AI notes.
    Notes,
    /// What the user typed.
    MyNotes,
    /// The action items.
    ActionItems,
    /// The answers to the user's insights (AI-06).
    Insights,
    /// The speaker-labelled transcript with timestamps.
    Transcript,
    /// Who attended.
    Attendees,
}

/// Which [`Section`]s an export includes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase", default)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "one switch per section, as the export dialog and settings show them"
)]
pub struct ExportSections {
    /// The AI notes.
    pub notes: bool,
    /// What the user typed.
    pub my_notes: bool,
    /// The action items.
    pub action_items: bool,
    /// Insights (AI-06).
    pub insights: bool,
    /// The transcript.
    pub transcript: bool,
    /// The attendee list.
    pub attendees: bool,
}

impl Default for ExportSections {
    /// Everything a reader of the notes needs; the transcript is opt-in.
    fn default() -> Self {
        Self {
            notes: true,
            my_notes: true,
            action_items: true,
            insights: true,
            transcript: false,
            attendees: true,
        }
    }
}

impl ExportSections {
    /// Every section.
    pub const ALL: Self = Self {
        notes: true,
        my_notes: true,
        action_items: true,
        insights: true,
        transcript: true,
        attendees: true,
    };

    /// Only the transcript.
    pub const TRANSCRIPT: Self = Self {
        notes: false,
        my_notes: false,
        action_items: false,
        insights: false,
        transcript: true,
        attendees: false,
    };

    /// What sharing the notes means: the notes, action items and attendees.
    pub const SHARE: Self = Self {
        notes: true,
        my_notes: false,
        action_items: true,
        insights: true,
        transcript: false,
        attendees: true,
    };

    /// True when the export includes `section`.
    pub const fn includes(self, section: Section) -> bool {
        match section {
            Section::Notes => self.notes,
            Section::MyNotes => self.my_notes,
            Section::ActionItems => self.action_items,
            Section::Insights => self.insights,
            Section::Transcript => self.transcript,
            Section::Attendees => self.attendees,
        }
    }
}

/// How people and tags are written in Markdown.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum LinkStyle {
    /// Names and tags as plain text.
    #[default]
    Plain,
    /// Obsidian: people as `[[Person]]` wiki links, tags as `#tags`, and
    /// frontmatter Obsidian reads as properties.
    Obsidian,
}

/// Everything a renderer needs besides the meeting.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RenderOptions {
    /// The sections to include (ignored by formats with a fixed content).
    pub sections: ExportSections,
    /// How people and tags are linked.
    pub links: LinkStyle,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_with_fixed_content_ignore_sections() {
        let fixed: Vec<ExportFormat> = ExportFormat::ALL
            .into_iter()
            .filter(|format| !format.honours_sections())
            .collect();
        assert_eq!(
            fixed,
            [ExportFormat::Json, ExportFormat::Srt, ExportFormat::Vtt]
        );
    }

    #[test]
    fn formats_serialize_as_their_snake_case_names() {
        let names = serde_json::to_value(ExportFormat::ALL).unwrap();
        assert_eq!(
            names,
            serde_json::json!(["markdown", "pdf", "docx", "text", "json", "srt", "vtt"])
        );
    }

    #[test]
    fn missing_section_switches_take_their_defaults() {
        let sections: ExportSections = serde_json::from_str(r#"{ "transcript": true }"#).unwrap();
        assert!(sections.transcript);
        assert!(sections.notes);
        assert!(sections.includes(Section::Attendees));
    }
}
