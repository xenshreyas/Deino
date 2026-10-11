//! Insight answers as typed values: what a validated run stores and what
//! the Insights tab, the global tables, exports and the CSV read.
//!
//! The card's output shape `[{value, speaker, timestamp_ms}]` is
//! [`InsightAnswer`]: `value` is one text, or one typed value per field.

use chrono::NaiveDate;
use deino_domain::{Millis, SegmentId};
use serde::{Deserialize, Serialize};

/// One field's value, of the field's type.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum FieldValue {
    /// Free text.
    Text(String),
    /// A finite number.
    Number(f64),
    /// Yes or no.
    Boolean(bool),
    /// A calendar day.
    Date(NaiveDate),
    /// One of an enum field's options, spelled as the definition spells it.
    Choice(String),
}

impl FieldValue {
    /// The value as a reader sees it: whole numbers without a decimal point,
    /// yes/no, ISO dates.
    pub fn display(&self) -> String {
        match self {
            Self::Text(text) | Self::Choice(text) => text.clone(),
            Self::Number(number) => format_number(*number),
            Self::Boolean(true) => "Yes".to_owned(),
            Self::Boolean(false) => "No".to_owned(),
            Self::Date(day) => day.to_string(),
        }
    }
}

/// `120` for 120.0, `4.5` for 4.5.
fn format_number(number: f64) -> String {
    if number.fract() == 0.0 && number.abs() < 1e15 {
        format!("{number:.0}")
    } else {
        number.to_string()
    }
}

/// One field of one answer; `None` when the meeting did not say, or the
/// model's value was not of the field's type.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct FieldEntry {
    /// The field's name, as the definition spells it.
    pub name: String,
    /// Its value.
    pub value: Option<FieldValue>,
}

/// What one answer says.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum InsightValue {
    /// One text, for an insight without fields.
    Text {
        /// The answer.
        text: String,
    },
    /// One entry per field, in the definition's order.
    Fields {
        /// The entries.
        fields: Vec<FieldEntry>,
    },
}

impl InsightValue {
    /// The answer on one line: the text, or `Field: value` pairs joined by
    /// `; ` (fields without a value left out).
    pub fn summary(&self) -> String {
        match self {
            Self::Text { text } => text.clone(),
            Self::Fields { fields } => fields
                .iter()
                .filter_map(|entry| {
                    let value = entry.value.as_ref()?;
                    Some(format!("{}: {}", entry.name, value.display()))
                })
                .collect::<Vec<_>>()
                .join("; "),
        }
    }

    /// The value of the field named `name`, if the answer has one.
    pub fn field(&self, name: &str) -> Option<&FieldValue> {
        match self {
            Self::Text { .. } => None,
            Self::Fields { fields } => fields
                .iter()
                .find(|entry| entry.name == name)
                .and_then(|entry| entry.value.as_ref()),
        }
    }
}

/// One answer of an insight on a meeting, attributed to the moment it came
/// from.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct InsightAnswer {
    /// What it says.
    pub value: InsightValue,
    /// Who said it. Stored as named when the answer was written; read back
    /// with the speaker's current name, so renames reach insights (CAP-04).
    pub speaker: Option<String>,
    /// Where it was said, from the start of the recording.
    pub timestamp_ms: Option<Millis>,
    /// The transcript segment it cites, for "show in transcript".
    pub segment_id: Option<SegmentId>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, value: Option<FieldValue>) -> FieldEntry {
        FieldEntry {
            name: name.into(),
            value,
        }
    }

    #[test]
    fn values_display_as_a_reader_expects() {
        let day = NaiveDate::from_ymd_opt(2026, 10, 8).unwrap();
        let cases = [
            (FieldValue::Number(120.0), "120"),
            (FieldValue::Number(4.5), "4.5"),
            (FieldValue::Number(-3.0), "-3"),
            (FieldValue::Boolean(true), "Yes"),
            (FieldValue::Boolean(false), "No"),
            (FieldValue::Date(day), "2026-10-08"),
            (FieldValue::Choice("Green".into()), "Green"),
            (FieldValue::Text("Okta SSO".into()), "Okta SSO"),
        ];
        for (value, expected) in cases {
            assert_eq!(value.display(), expected, "{value:?}");
        }
    }

    #[test]
    fn a_summary_lists_the_filled_fields_only() {
        let value = InsightValue::Fields {
            fields: vec![
                entry("Budget", Some(FieldValue::Number(40_000.0))),
                entry("Timeline", None),
                entry("Health", Some(FieldValue::Choice("Green".into()))),
            ],
        };

        assert_eq!(value.summary(), "Budget: 40000; Health: Green");
        assert_eq!(
            value.field("Health"),
            Some(&FieldValue::Choice("Green".into()))
        );
        assert_eq!(value.field("Timeline"), None);
    }

    #[test]
    fn answers_serialize_as_tagged_values_with_camel_case_attribution() {
        let answer = InsightAnswer {
            value: InsightValue::Fields {
                fields: vec![entry("Score", Some(FieldValue::Number(4.0)))],
            },
            speaker: Some("Maya Chen".into()),
            timestamp_ms: Some(Millis(271_000)),
            segment_id: None,
        };

        let json = serde_json::to_value(&answer).unwrap();

        assert_eq!(
            json,
            serde_json::json!({
                "value": { "kind": "fields", "fields": [
                    { "name": "Score", "value": { "type": "number", "value": 4.0 } }
                ] },
                "speaker": "Maya Chen",
                "timestampMs": 271_000,
                "segmentId": null
            })
        );
        assert_eq!(
            serde_json::from_value::<InsightAnswer>(json).unwrap(),
            answer
        );
    }
}
