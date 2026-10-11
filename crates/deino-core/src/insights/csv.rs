//! An insight's table as CSV, through `deino-export`'s CSV writer: one
//! column per field (or one "Answer"), one row per answer, saved where the
//! user picks in the save dialog.

use std::path::PathBuf;

use deino_domain::InsightId;
use deino_export::{InsightRowExport, InsightTableExport, csv};

use super::{InsightDef, InsightError, InsightService, InsightValue, MeetingRun};
use crate::export::{SaveRequest, SavedExport};

/// The value column of an insight without fields.
const ANSWER_COLUMN: &str = "Answer";

impl InsightService {
    /// Saves the insight's table as CSV; `None` when the user cancels.
    pub async fn export_csv(&self, id: &InsightId) -> Result<Option<SavedExport>, InsightError> {
        let table = self.table(id).await?;
        let request = SaveRequest {
            file_name: format!("{}.csv", file_stem(&table.def.name)),
            kind: "CSV table",
            extension: "csv",
        };
        let Some(chosen) = self.ports.files.save_dialog.save_path(request).await else {
            return Ok(None);
        };
        let path = with_csv_extension(chosen);
        let export = self.table_export(&table.def, &table.rows);
        self.ports
            .files
            .vault
            .write(&path, csv::insight_table(&export).into_bytes())
            .await
            .map_err(InsightError::Write)?;
        let file_name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        Ok(Some(SavedExport { file_name }))
    }

    /// The table as the CSV writer reads it: values written out, meeting
    /// times in the user's zone.
    pub fn table_export(&self, def: &InsightDef, rows: &[MeetingRun]) -> InsightTableExport {
        let columns: Vec<String> = if def.is_free_text() {
            vec![ANSWER_COLUMN.to_owned()]
        } else {
            def.fields.iter().map(|field| field.name.clone()).collect()
        };
        let zone = self.ports.files.zone.as_ref();
        let rows = rows
            .iter()
            .flat_map(|row| {
                row.run.answers.iter().map(move |answer| InsightRowExport {
                    meeting_id: row.run.meeting_id.clone(),
                    meeting_title: row.meeting_title.clone(),
                    started_at: zone.local(row.meeting_started_at),
                    cells: cells(def, &answer.value),
                    speaker: answer.speaker.clone(),
                    at: answer.timestamp_ms,
                })
            })
            .collect();
        InsightTableExport { columns, rows }
    }
}

/// `path`, ending in `.csv`.
fn with_csv_extension(path: PathBuf) -> PathBuf {
    if path
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("csv"))
    {
        return path;
    }
    let mut name = path.into_os_string();
    name.push(".csv");
    name.into()
}

/// One written-out cell per column.
fn cells(def: &InsightDef, value: &InsightValue) -> Vec<String> {
    match value {
        InsightValue::Text { text } => vec![text.clone()],
        InsightValue::Fields { .. } => def
            .fields
            .iter()
            .map(|field| {
                value
                    .field(&field.name)
                    .map(super::FieldValue::display)
                    .unwrap_or_default()
            })
            .collect(),
    }
}

/// A file name from an insight's name: characters file systems refuse
/// become spaces.
fn file_stem(name: &str) -> String {
    let stem: String = name
        .chars()
        .map(|character| match character {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => ' ',
            character if character.is_control() => ' ',
            character => character,
        })
        .collect();
    let stem = stem.trim();
    if stem.is_empty() {
        "Insight".to_owned()
    } else {
        stem.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_stems_drop_what_file_systems_refuse() {
        assert_eq!(file_stem("Deal summary: CRM/Q4"), "Deal summary  CRM Q4");
        assert_eq!(file_stem("  ??  "), "Insight");
    }
}
