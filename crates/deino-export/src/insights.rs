//! Insights in exports (AI-06): a meeting's answers as an export section,
//! and one insight's answers across meetings as a CSV table.
//!
//! Values arrive already written out (`"40000"`, `"Yes"`, `"2026-10-08"`),
//! so every format shows a field the same way. Each format lays the section
//! out its own way from [`InsightAnswerExport::line`] and
//! [`InsightAnswerExport::attribution`].

use chrono::{DateTime, FixedOffset};
use deino_domain::{MeetingId, Millis};
use serde::Serialize;

use crate::labels;

/// One insight's answers on the meeting.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InsightExport {
    /// The insight's name.
    pub name: String,
    /// Its answers, in the order the meeting gave them.
    pub answers: Vec<InsightAnswerExport>,
}

/// One answer, attributed to the moment it came from.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InsightAnswerExport {
    /// What it says.
    pub value: InsightExportValue,
    /// Who said it.
    pub speaker: Option<String>,
    /// When, from the start of the recording.
    pub at: Option<Millis>,
}

/// An answer's value: one text, or written-out field values.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum InsightExportValue {
    /// One text.
    Text {
        /// The answer.
        text: String,
    },
    /// The fields that have a value, in the insight's order.
    Fields {
        /// Name and written-out value of each.
        fields: Vec<ExportField>,
    },
}

/// One field of an answer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ExportField {
    /// The field's name.
    pub name: String,
    /// Its value, written out.
    pub value: String,
}

impl InsightAnswerExport {
    /// The answer on one line: the text, or `Name: value · Name: value`.
    pub(crate) fn line(&self) -> String {
        match &self.value {
            InsightExportValue::Text { text } => text.trim().to_owned(),
            InsightExportValue::Fields { fields } => fields
                .iter()
                .map(|field| format!("{}: {}", field.name, field.value))
                .collect::<Vec<_>>()
                .join(" · "),
        }
    }

    /// `Maya Chen, 04:31`, whichever parts are known.
    pub(crate) fn attribution(&self) -> Option<String> {
        let parts: Vec<String> = self
            .speaker
            .clone()
            .into_iter()
            .chain(self.at.map(labels::moment))
            .collect();
        (!parts.is_empty()).then(|| parts.join(", "))
    }
}

/// One insight's answers across meetings: the Insights page's table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InsightTableExport {
    /// The value columns: the field names, or one "Answer" column.
    pub columns: Vec<String>,
    /// One row per answer, newest meeting first.
    pub rows: Vec<InsightRowExport>,
}

/// One answer in the table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InsightRowExport {
    /// The meeting it came from.
    pub meeting_id: MeetingId,
    /// The meeting's title.
    pub meeting_title: String,
    /// When the meeting started, in the user's zone.
    pub started_at: DateTime<FixedOffset>,
    /// One written-out value per column; empty when the answer has none.
    pub cells: Vec<String>,
    /// Who said it.
    pub speaker: Option<String>,
    /// When, from the start of the recording.
    pub at: Option<Millis>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn answer(
        value: InsightExportValue,
        speaker: Option<&str>,
        at: Option<u32>,
    ) -> InsightAnswerExport {
        InsightAnswerExport {
            value,
            speaker: speaker.map(str::to_owned),
            at: at.map(Millis),
        }
    }

    #[test]
    fn an_answer_reads_as_one_line_with_its_attribution() {
        let fields = InsightExportValue::Fields {
            fields: vec![
                ExportField {
                    name: "Budget".into(),
                    value: "40000".into(),
                },
                ExportField {
                    name: "Health".into(),
                    value: "Green".into(),
                },
            ],
        };
        let text = InsightExportValue::Text {
            text: " Okta SSO is required. ".into(),
        };

        let full = answer(fields, Some("Owen Hart"), Some(271_000));
        let bare = answer(text, None, None);

        assert_eq!(full.line(), "Budget: 40000 · Health: Green");
        assert_eq!(full.attribution().as_deref(), Some("Owen Hart, 04:31"));
        assert_eq!(bare.line(), "Okta SSO is required.");
        assert_eq!(bare.attribution(), None);
    }
}
