//! Validation: the model's wire answer → typed answers, field by field.
//!
//! Every value is checked against its field's type. A value that is close
//! enough is repaired (`"$40k"` → 40000, `"yes"` → true, `"green"` →
//! `Green`, `"October 8, 2026"` → 2026-10-08); one that is not is dropped
//! (the field stays empty). An answer with nothing valid left is dropped,
//! repeats are dropped, and the rest are cut to the insight's count.

use chrono::NaiveDate;

use super::wire::{WireAnswer, WireInstance, WireScalar};
use super::{Cardinality, FieldEntry, FieldKind, FieldValue, InsightField, InsightValue};

/// The longest text value kept, in characters; longer ones are cut.
pub const MAX_VALUE_CHARS: usize = 4_000;
/// Date spellings accepted besides ISO 8601.
const DATE_FORMATS: [&str; 6] = [
    "%Y-%m-%d",
    "%B %d, %Y",
    "%b %d, %Y",
    "%d %B %Y",
    "%d %b %Y",
    "%B %d %Y",
];
/// Words a yes/no field reads as yes.
const YES: [&str; 5] = ["yes", "y", "true", "1", "yep"];
/// Words a yes/no field reads as no.
const NO: [&str; 5] = ["no", "n", "false", "0", "nope"];

/// One answer that passed validation, with the transcript references the
/// model gave for it (resolved later by [`super::Attribution`]).
#[derive(Clone, Debug, PartialEq)]
pub struct Validated {
    /// The typed answer.
    pub value: InsightValue,
    /// `s12`-style references, in the model's order.
    pub references: Vec<String>,
}

/// The valid answers of `wire` for an insight with `fields` (none: free
/// text), without repeats, at most `cardinality` of them.
pub fn validate(
    wire: &WireAnswer,
    fields: &[InsightField],
    cardinality: Cardinality,
) -> Vec<Validated> {
    let mut kept: Vec<Validated> = Vec::new();
    for instance in &wire.answers {
        let Some(value) = answer_value(instance, fields) else {
            continue;
        };
        if kept.iter().any(|earlier| earlier.value == value) {
            continue;
        }
        kept.push(Validated {
            value,
            references: references(instance),
        });
        if kept.len() == cardinality.limit() {
            break;
        }
    }
    kept
}

fn answer_value(instance: &WireInstance, fields: &[InsightField]) -> Option<InsightValue> {
    if fields.is_empty() {
        let text = instance.value.as_ref().and_then(text_of)?;
        return Some(InsightValue::Text { text });
    }
    let entries: Vec<FieldEntry> = fields
        .iter()
        .map(|field| FieldEntry {
            name: field.name.clone(),
            value: raw_field(instance, &field.name).and_then(|raw| coerce(&field.kind, raw)),
        })
        .collect();
    entries
        .iter()
        .any(|entry| entry.value.is_some())
        .then_some(InsightValue::Fields { fields: entries })
}

/// The model's value for `name`: under its exact name, else under the same
/// name in another case or with stray spaces.
fn raw_field<'a>(instance: &'a WireInstance, name: &str) -> Option<&'a WireScalar> {
    if let Some(raw) = instance.fields.get(name) {
        return raw.as_ref();
    }
    let wanted = name.trim().to_lowercase();
    instance
        .fields
        .iter()
        .find(|(key, _)| key.trim().to_lowercase() == wanted)
        .and_then(|(_, raw)| raw.as_ref())
}

fn references(instance: &WireInstance) -> Vec<String> {
    instance
        .evidence
        .as_ref()
        .map(|evidence| {
            evidence
                .references()
                .into_iter()
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

/// `raw` as a value of `kind`, repaired when it is close; `None` otherwise.
pub fn coerce(kind: &FieldKind, raw: &WireScalar) -> Option<FieldValue> {
    match kind {
        FieldKind::Text => text_of(raw).map(FieldValue::Text),
        FieldKind::Number => number_of(raw).map(FieldValue::Number),
        FieldKind::Boolean => boolean_of(raw).map(FieldValue::Boolean),
        FieldKind::Date => date_of(raw).map(FieldValue::Date),
        FieldKind::Enum { options } => choice_of(options, raw).map(FieldValue::Choice),
    }
}

/// Text: trimmed and non-empty; numbers and yes/no written out; a list
/// joined with `; `.
fn text_of(raw: &WireScalar) -> Option<String> {
    let text = match raw {
        WireScalar::Text(text) => text.trim().to_owned(),
        WireScalar::Number(number) => FieldValue::Number(*number).display(),
        WireScalar::Bool(flag) => FieldValue::Boolean(*flag).display(),
        WireScalar::List(items) => items
            .iter()
            .filter_map(text_of)
            .collect::<Vec<_>>()
            .join("; "),
    };
    (!text.is_empty()).then(|| text.chars().take(MAX_VALUE_CHARS).collect())
}

/// A finite number, from a number or a string such as `$40,000`, `40k`,
/// `12.5%` or `~120`.
fn number_of(raw: &WireScalar) -> Option<f64> {
    match raw {
        WireScalar::Number(number) => number.is_finite().then_some(*number),
        WireScalar::Text(text) => parse_number(text),
        WireScalar::List(items) => items.first().and_then(number_of),
        WireScalar::Bool(_) => None,
    }
}

fn parse_number(text: &str) -> Option<f64> {
    let cleaned: String = text
        .trim()
        .trim_start_matches(['~', '≈'])
        .trim_end_matches('%')
        .chars()
        .filter(|character| !matches!(character, '$' | '€' | '£' | '¥' | ',' | '_' | ' '))
        .collect();
    let (digits, scale) = match cleaned.char_indices().last() {
        Some((at, 'k' | 'K')) => (cleaned.get(..at)?, 1e3),
        Some((at, 'm' | 'M')) => (cleaned.get(..at)?, 1e6),
        Some((at, 'b' | 'B')) => (cleaned.get(..at)?, 1e9),
        _ => (cleaned.as_str(), 1.0),
    };
    let number = digits.parse::<f64>().ok()? * scale;
    number.is_finite().then_some(number)
}

/// Yes or no, from a boolean, 1/0 or a yes/no word.
fn boolean_of(raw: &WireScalar) -> Option<bool> {
    match raw {
        WireScalar::Bool(flag) => Some(*flag),
        WireScalar::Number(number) if *number == 1.0 => Some(true),
        WireScalar::Number(number) if *number == 0.0 => Some(false),
        WireScalar::Text(text) => {
            let word = text.trim().trim_end_matches(['.', '!']).to_lowercase();
            match word.as_str() {
                word if YES.contains(&word) => Some(true),
                word if NO.contains(&word) => Some(false),
                _ => None,
            }
        }
        WireScalar::List(items) => items.first().and_then(boolean_of),
        WireScalar::Number(_) => None,
    }
}

/// A day, from ISO 8601 (a date-time keeps its date) or a written-out date.
fn date_of(raw: &WireScalar) -> Option<NaiveDate> {
    let text = match raw {
        WireScalar::Text(text) => text.trim(),
        WireScalar::List(items) => return items.first().and_then(date_of),
        WireScalar::Number(_) | WireScalar::Bool(_) => return None,
    };
    let iso = text.get(..10).filter(|_| text.contains('T'));
    DATE_FORMATS.iter().find_map(|format| {
        NaiveDate::parse_from_str(text, format)
            .ok()
            .or_else(|| iso.and_then(|day| NaiveDate::parse_from_str(day, format).ok()))
    })
}

/// The option `raw` names: exactly, else ignoring case, else ignoring case
/// and punctuation (`"green."` → `Green`).
fn choice_of(options: &[String], raw: &WireScalar) -> Option<String> {
    let text = text_of(raw)?;
    let found = options
        .iter()
        .find(|option| **option == text)
        .or_else(|| {
            options
                .iter()
                .find(|option| option.eq_ignore_ascii_case(&text))
        })
        .or_else(|| {
            let wanted = letters(&text);
            options.iter().find(|option| letters(option) == wanted)
        })?;
    Some(found.clone())
}

/// The lower-case letters and digits of `text`.
fn letters(text: &str) -> String {
    text.chars()
        .filter(|character| character.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

#[cfg(test)]
#[path = "validate_tests.rs"]
mod tests;
