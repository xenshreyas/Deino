//! [`LegacySummaryImport`]: the summaries written before NOT-02 become
//! version 1 of each meeting's enhanced notes, before the migration that
//! drops `summary_processes` runs (a `deino_db::DataMigration`).
//!
//! A stored summary is JSON in one of two shapes: `{ "markdown": … }` (every
//! summary since FND-09, and the `BlockNote` editor's saves, which kept the
//! Markdown beside its blocks) or the first app's sections
//! (`{ "Key": { "title", "blocks": [{ "type", "content" }] }, "_section_order" }`).
//! Its text is marked as the AI's. A meeting that already has enhanced
//! notes is skipped, so a crash between this step and the drop is harmless.

use async_trait::async_trait;
use deino_db::{DataMigration, DataMigrationError, Database};
use deino_doc::{Block, Doc, Flavor, Inline, Mark, OriginAttrs};
use serde_json::Value;

use super::LEGACY_MODEL;

/// `20261011180916_not02_drop_legacy_summaries.sql`.
pub const DROP_LEGACY_SUMMARIES_VERSION: i64 = 20_261_011_180_916;

const SUMMARIES: &str = "SELECT meeting_id, result, updated_at FROM summary_processes
     WHERE result IS NOT NULL AND meeting_id IN (SELECT id FROM meetings)
       AND meeting_id NOT IN (SELECT meeting_id FROM enhanced_notes)";
const INSERT: &str = "INSERT INTO enhanced_notes (meeting_id, version, doc_json, markdown,
     template_id, model, basis_json, is_user_edited, created_at, updated_at)
     VALUES (?1, 1, ?2, ?3, NULL, ?4, NULL, 0, ?5, ?5)";

/// Copies the legacy summaries into `enhanced_notes` (see the module docs).
#[derive(Clone, Copy, Debug, Default)]
pub struct LegacySummaryImport;

#[async_trait]
impl DataMigration for LegacySummaryImport {
    fn runs_before(&self) -> i64 {
        DROP_LEGACY_SUMMARIES_VERSION
    }

    async fn run(&self, database: &Database) -> Result<(), DataMigrationError> {
        let rows: Vec<(String, String, String)> = sqlx::query_as(SUMMARIES)
            .fetch_all(database.writer())
            .await?;
        let mut imported = 0;
        for (meeting, result, updated_at) in rows {
            let Some(doc) = summary_doc(&result) else {
                continue;
            };
            let markdown = deino_doc::to_markdown(&doc, Flavor::Readable);
            sqlx::query(INSERT)
                .bind(&meeting)
                .bind(serde_json::to_string(&doc)?)
                .bind(markdown)
                .bind(LEGACY_MODEL)
                .bind(updated_at)
                .execute(database.writer())
                .await?;
            imported += 1;
        }
        log::info!(target: "enhance", "imported {imported} legacy summaries as enhanced notes");
        Ok(())
    }
}

/// The summary stored as `result`, as an AI-written document; `None` when
/// it is empty or in no known shape.
pub fn summary_doc(result: &str) -> Option<Doc> {
    let value: Value = serde_json::from_str(result).ok()?;
    let markdown = match value.get("markdown").and_then(Value::as_str) {
        Some(markdown) => markdown.to_owned(),
        None => sections_markdown(&value),
    };
    if markdown.trim().is_empty() {
        return None;
    }
    let mut doc = deino_doc::from_markdown(&markdown).ok()?;
    doc.content.iter_mut().for_each(mark_block);
    Some(doc.normalized())
}

/// The first app's section JSON as Markdown, one `##` section each.
fn sections_markdown(value: &Value) -> String {
    let Some(object) = value.as_object() else {
        return String::new();
    };
    let order: Vec<String> = match object.get("_section_order").and_then(Value::as_array) {
        Some(order) => order
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect(),
        None => object.keys().cloned().collect(),
    };
    let sections: Vec<String> = order
        .iter()
        .filter_map(|key| section_markdown(key, object.get(key)?))
        .collect();
    sections.join("\n\n")
}

fn section_markdown(key: &str, section: &Value) -> Option<String> {
    let blocks = section.get("blocks")?.as_array()?;
    let title = section.get("title").and_then(Value::as_str).unwrap_or(key);
    let lines: Vec<String> = blocks
        .iter()
        .filter_map(|block| {
            let content = block.get("content")?.as_str()?;
            let kind = block
                .get("type")
                .and_then(Value::as_str)
                .unwrap_or_default();
            Some(match kind {
                "bullet" => format!("- {content}"),
                kind if kind.starts_with("heading") => format!("### {content}"),
                _ => content.to_owned(),
            })
        })
        .collect();
    Some(format!("## {title}\n\n{}", lines.join("\n")))
}

fn mark_block(block: &mut Block) {
    match block {
        Block::Paragraph { content } | Block::Heading { content, .. } => {
            content.iter_mut().for_each(mark_inline);
        }
        Block::BulletList { content } | Block::OrderedList { content, .. } => {
            for item in content {
                item.content.iter_mut().for_each(mark_block);
            }
        }
        Block::TaskList { content } => {
            for item in content {
                item.content.iter_mut().for_each(mark_block);
            }
        }
        Block::Blockquote { content } => content.iter_mut().for_each(mark_block),
        Block::CodeBlock { .. } | Block::HorizontalRule | Block::Image { .. } => {}
    }
}

fn mark_inline(inline: &mut Inline) {
    let marks = match inline {
        Inline::Text { marks, .. }
        | Inline::HardBreak { marks }
        | Inline::Mention { marks, .. }
        | Inline::Citation { marks, .. } => marks,
    };
    if !marks.iter().any(|mark| matches!(mark, Mark::Origin { .. })) {
        marks.insert(
            0,
            Mark::Origin {
                attrs: OriginAttrs::AI,
            },
        );
    }
}

#[cfg(test)]
mod legacy_import_tests;
