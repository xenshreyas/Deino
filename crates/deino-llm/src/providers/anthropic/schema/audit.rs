//! A test helper: proves a real schema reaches Anthropic constrained, and
//! that what is sent keeps to the structured-outputs subset.

use serde_json::{Map, Value};

use super::{FORMATS, OutputSchema, STRIPPED, is_object_schema, output_schema, subschemas};

/// The schema Anthropic would receive for `schema`, or why it would not be
/// constrained or would break a structured-outputs rule.
///
/// # Errors
///
/// When `schema` falls back to an unconstrained answer, or the rewrite left
/// an object without `"additionalProperties": false`, a refused keyword, an
/// unsupported `format` or a `oneOf`.
pub fn constrained_schema(schema: &Value) -> Result<Value, String> {
    let sent = match output_schema(schema) {
        OutputSchema::Constrained(sent) => sent,
        OutputSchema::Unconstrained(reason) => {
            return Err(format!("the schema is {reason}"));
        }
    };
    audit(&sent)?;
    Ok(sent)
}

fn audit(node: &Value) -> Result<(), String> {
    let Value::Object(map) = node else {
        return Ok(());
    };
    match problem(map) {
        Some(problem) => Err(format!("{problem} in {node}")),
        None => subschemas(map, true).into_iter().try_for_each(audit),
    }
}

fn problem(map: &Map<String, Value>) -> Option<String> {
    if is_object_schema(map) && map.get("additionalProperties") != Some(&Value::Bool(false)) {
        return Some("an object without additionalProperties: false".to_owned());
    }
    if let Some(keyword) = STRIPPED.iter().find(|keyword| map.contains_key(**keyword)) {
        return Some(format!("the refused keyword {keyword}"));
    }
    if let Some(format) = map.get("format").and_then(Value::as_str)
        && !FORMATS.contains(&format)
    {
        return Some(format!("the unsupported format {format}"));
    }
    map.contains_key("oneOf").then(|| "a oneOf".to_owned())
}
