//! Template files: export and import as JSON, including the pre-NOT-04
//! format that custom templates were written in.
//!
//! An export is a [`TemplateDocument`] (`"format": "deino.template"`,
//! `"version": 1`). Import accepts that, or a legacy file (`name`,
//! `description`, `sections[{title, instruction, format: paragraph|list|string,
//! item_format?}]`), which it converts: lists with a Markdown table hint
//! become tables, the hint's column names move into the instructions.

use deino_domain::SectionFormat;
use serde::{Deserialize, Serialize};

use super::{TemplateContent, TemplateError, TemplateSection};

/// The `format` of an exported template file.
pub const DOCUMENT_FORMAT: &str = "deino.template";
/// The version of the exported file format.
pub const DOCUMENT_VERSION: u32 = 1;

/// An exported template.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemplateDocument {
    /// Always [`DOCUMENT_FORMAT`].
    pub format: String,
    /// Always [`DOCUMENT_VERSION`] for now.
    pub version: u32,
    /// The template.
    pub template: TemplateContent,
}

/// `content` as a pretty-printed template file.
pub fn export_document(content: &TemplateContent) -> Result<String, TemplateError> {
    let document = TemplateDocument {
        format: DOCUMENT_FORMAT.to_owned(),
        version: DOCUMENT_VERSION,
        template: content.clone(),
    };
    serde_json::to_string_pretty(&document).map_err(TemplateError::UnreadableDocument)
}

/// The template in a file, current or legacy, before validation.
pub fn import_document(json: &str) -> Result<TemplateContent, TemplateError> {
    match serde_json::from_str::<TemplateDocument>(json) {
        Ok(document) if document.format == DOCUMENT_FORMAT => Ok(document.template),
        Ok(_) => Err(TemplateError::invalid("file", "is not a Deino template")),
        Err(error) => serde_json::from_str::<LegacyTemplate>(json)
            .map(LegacyTemplate::into_content)
            .map_err(|_| TemplateError::UnreadableDocument(error)),
    }
}

/// A template file as the pre-NOT-04 loader read it.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyTemplate {
    name: String,
    #[serde(default)]
    description: String,
    sections: Vec<LegacySection>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacySection {
    title: String,
    instruction: String,
    format: String,
    item_format: Option<String>,
    example_item_format: Option<String>,
}

impl LegacyTemplate {
    fn into_content(self) -> TemplateContent {
        TemplateContent {
            name: self.name,
            description: self.description,
            meeting_context: String::new(),
            sections: self
                .sections
                .into_iter()
                .map(LegacySection::into_section)
                .collect(),
            auto_rules: Vec::new(),
        }
    }
}

impl LegacySection {
    fn into_section(self) -> TemplateSection {
        let columns = self
            .item_format
            .or(self.example_item_format)
            .map(|hint| table_columns(&hint))
            .filter(|columns| !columns.is_empty());
        let format = match (self.format.as_str(), &columns) {
            ("list", Some(_)) => SectionFormat::Table,
            ("list", None) => SectionFormat::Bullets,
            _ => SectionFormat::Paragraph,
        };
        let instructions = match columns {
            Some(columns) => format!(
                "{}. Columns: {}.",
                self.instruction.trim_end_matches('.'),
                columns.join(", ")
            ),
            None => self.instruction,
        };
        TemplateSection {
            title: self.title,
            instructions,
            format,
        }
    }
}

/// The column names of a Markdown table hint's header row
/// (`| **Owner** | Task |` → `Owner`, `Task`).
fn table_columns(hint: &str) -> Vec<String> {
    hint.lines()
        .next()
        .unwrap_or_default()
        .split('|')
        .map(|cell| cell.trim().trim_matches('*').trim().to_owned())
        .filter(|cell| !cell.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const LEGACY: &str = r#"{
        "name": "Daily Standup",
        "description": "Time-boxed daily updates.",
        "sections": [
            { "title": "Date", "instruction": "YYYY-MM-DD", "format": "string" },
            { "title": "Blockers", "instruction": "Any impediments", "format": "list",
              "item_format": "| **Owner** | **Blocker** |\n| --- | --- |" },
            { "title": "Attendees", "instruction": "Who came", "format": "list" }
        ]
    }"#;

    #[test]
    fn legacy_files_convert_formats_and_table_columns() {
        let content = import_document(LEGACY).unwrap();

        let formats: Vec<SectionFormat> = content.sections.iter().map(|s| s.format).collect();
        assert_eq!(
            formats,
            [
                SectionFormat::Paragraph,
                SectionFormat::Table,
                SectionFormat::Bullets
            ]
        );
        assert_eq!(
            content.sections[1].instructions,
            "Any impediments. Columns: Owner, Blocker."
        );
        assert!(content.validated().is_ok());
    }

    #[test]
    fn exports_import_back_unchanged() {
        let content = import_document(LEGACY).unwrap();

        let exported = export_document(&content).unwrap();

        assert!(exported.contains(r#""format": "deino.template""#));
        assert_eq!(import_document(&exported).unwrap(), content);
    }

    #[test]
    fn anything_else_is_rejected_as_not_a_template() {
        let error = import_document(r#"{ "hello": 1 }"#).unwrap_err();

        assert_eq!(error.to_string(), "this file is not a Deino template");
    }
}
