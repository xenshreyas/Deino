//! The JSON schema an insight's answer is held to, built from its fields at
//! run time (the gateway's `structured_as`).
//!
//! Draft-07, objects and arrays only, the same subset `schemars` produces
//! for the built-in routes, so every provider accepts it: Anthropic as a
//! forced tool, OpenAI-compatible as `response_format`, the built-in model as
//! a grammar. This is the one place insights build JSON by hand; the answer
//! comes back as the typed [`WireAnswer`](super::wire::WireAnswer).

use serde_json::{Value, json};

use super::{Cardinality, FieldKind, InsightField};

/// What `evidence` holds, for the schema's description.
const EVIDENCE: &str = "The id of the transcript line the answer comes from, for example \"s12\".";

/// The schema of an answer to an insight with `fields` (none: free text).
pub fn answer_schema(fields: &[InsightField], cardinality: Cardinality) -> Value {
    let mut answers = json!({ "type": "array", "items": instance_schema(fields) });
    if let (Cardinality::Fixed { count }, Some(object)) = (cardinality, answers.as_object_mut()) {
        object.insert("maxItems".to_owned(), json!(count));
    }
    json!({
        "type": "object",
        "properties": { "answers": answers },
        "required": ["answers"],
        "additionalProperties": false
    })
}

fn instance_schema(fields: &[InsightField]) -> Value {
    let evidence = json!({ "type": "string", "description": EVIDENCE });
    if fields.is_empty() {
        return json!({
            "type": "object",
            "properties": {
                "value": { "type": "string", "description": "The answer, as plain text." },
                "evidence": evidence
            },
            "required": ["value", "evidence"],
            "additionalProperties": false
        });
    }
    let properties: serde_json::Map<String, Value> = fields
        .iter()
        .map(|field| (field.name.clone(), field_schema(&field.kind)))
        .collect();
    let names: Vec<&str> = fields.iter().map(|field| field.name.as_str()).collect();
    json!({
        "type": "object",
        "properties": {
            "fields": {
                "type": "object",
                "properties": properties,
                "required": names,
                "additionalProperties": false
            },
            "evidence": evidence
        },
        "required": ["fields", "evidence"],
        "additionalProperties": false
    })
}

/// A field's value: its JSON type, or null when the meeting did not say.
fn field_schema(kind: &FieldKind) -> Value {
    match kind {
        FieldKind::Text => json!({ "type": ["string", "null"] }),
        FieldKind::Number => json!({ "type": ["number", "null"] }),
        FieldKind::Boolean => json!({ "type": ["boolean", "null"] }),
        FieldKind::Date => json!({
            "type": ["string", "null"],
            "description": "A date as YYYY-MM-DD."
        }),
        FieldKind::Enum { options } => {
            let mut choices: Vec<Value> = options.iter().map(|option| json!(option)).collect();
            choices.push(Value::Null);
            json!({ "enum": choices })
        }
    }
}

/// The prompt's field list: one `- Name: type` line per field; `None`
/// for free-text answers.
pub fn field_lines(fields: &[InsightField]) -> Option<String> {
    if fields.is_empty() {
        return None;
    }
    let lines: Vec<String> = fields
        .iter()
        .map(|field| match &field.kind {
            FieldKind::Enum { options } => {
                format!("- {}: one of {}", field.name, options.join(", "))
            }
            kind => format!("- {}: {}", field.name, kind.label()),
        })
        .collect();
    Some(lines.join("\n"))
}

/// The prompt's answer count.
pub fn count_line(cardinality: Cardinality) -> String {
    match cardinality {
        Cardinality::Auto => "as many as the meeting holds".to_owned(),
        Cardinality::Fixed { count } => format!("at most {count}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fields() -> Vec<InsightField> {
        vec![
            InsightField {
                name: "Budget".into(),
                kind: FieldKind::Number,
            },
            InsightField {
                name: "Health".into(),
                kind: FieldKind::Enum {
                    options: vec!["Red".into(), "Green".into()],
                },
            },
        ]
    }

    #[test]
    fn a_field_schema_types_every_field_and_caps_a_fixed_count() {
        let schema = answer_schema(&fields(), Cardinality::Fixed { count: 1 });

        let answers = &schema["properties"]["answers"];
        assert_eq!(answers["maxItems"], 1);
        let fields = &answers["items"]["properties"]["fields"];
        assert_eq!(fields["required"], json!(["Budget", "Health"]));
        assert_eq!(
            fields["properties"]["Budget"]["type"],
            json!(["number", "null"])
        );
        assert_eq!(
            fields["properties"]["Health"]["enum"],
            json!(["Red", "Green", null])
        );
        assert_eq!(answers["items"]["required"], json!(["fields", "evidence"]));
    }

    #[test]
    fn a_free_text_schema_asks_for_a_value_and_has_no_cap_on_auto() {
        let schema = answer_schema(&[], Cardinality::Auto);

        let answers = &schema["properties"]["answers"];
        assert!(answers.get("maxItems").is_none());
        assert_eq!(answers["items"]["required"], json!(["value", "evidence"]));
    }

    #[test]
    fn the_prompt_lines_name_each_field_and_the_count() {
        assert_eq!(
            field_lines(&fields()).as_deref(),
            Some("- Budget: number\n- Health: one of Red, Green")
        );
        assert_eq!(field_lines(&[]), None);
        assert_eq!(count_line(Cardinality::Fixed { count: 3 }), "at most 3");
        assert_eq!(
            count_line(Cardinality::Auto),
            "as many as the meeting holds"
        );
    }
}
