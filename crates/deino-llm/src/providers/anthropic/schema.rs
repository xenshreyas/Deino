//! The JSON schema Anthropic's structured outputs receive
//! (`output_config.format`), rewritten from the answer's draft-07 schema.
//!
//! Structured outputs accept a subset of JSON Schema: every object must say
//! `"additionalProperties": false`; numeric bounds, string lengths, array
//! sizes and most string `format`s are refused; recursive schemas are
//! refused. The official SDKs strip what the API refuses and validate the
//! answer on the client, and so does Deino: [`output_schema`] is that
//! rewrite, and `structured.rs` still parses (and repairs) every answer.
//!
//! A schema the API cannot take at all (recursive, a `$ref` that does not
//! resolve, an open map, an "any value" subschema) is reported as
//! [`OutputSchema::Unconstrained`]; the request then asks for JSON in the
//! system prompt instead, and the client-side parse is the only check.

mod refs;

use std::fmt;

use serde_json::{Map, Value};

use self::refs::check_refs;

/// Keywords structured outputs refuse; dropped, and checked on the client.
const STRIPPED: [&str; 15] = [
    "minimum",
    "maximum",
    "exclusiveMinimum",
    "exclusiveMaximum",
    "multipleOf",
    "minLength",
    "maxLength",
    "minItems",
    "maxItems",
    "uniqueItems",
    "contains",
    "minContains",
    "maxContains",
    "minProperties",
    "maxProperties",
];

/// The string formats structured outputs accept; any other `format` is dropped.
const FORMATS: [&str; 10] = [
    "date-time",
    "time",
    "date",
    "duration",
    "email",
    "hostname",
    "uri",
    "ipv4",
    "ipv6",
    "uuid",
];

/// Keys whose value maps names to subschemas.
const SCHEMA_MAPS: [&str; 3] = ["properties", "$defs", "definitions"];
/// Keys whose value is a list of subschemas.
const SCHEMA_LISTS: [&str; 4] = ["anyOf", "allOf", "oneOf", "prefixItems"];
/// Keys whose value is one subschema (`items` may also be a list).
const SCHEMA_VALUES: [&str; 6] = ["items", "additionalProperties", "not", "if", "then", "else"];

/// What Anthropic is sent for a JSON answer's schema.
#[derive(Clone, Debug, PartialEq)]
pub enum OutputSchema {
    /// The schema rewritten to the structured-outputs subset.
    Constrained(Value),
    /// The schema cannot be expressed; ask for JSON in the prompt instead.
    Unconstrained(Unsupported),
}

/// Why a schema cannot go to structured outputs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unsupported {
    /// A definition refers back to itself, directly or through others.
    Recursive,
    /// A `$ref` that is not `#` or a root `$defs`/`definitions` entry.
    UnresolvedRef,
    /// An object with extra properties of its own (a map), which structured
    /// outputs cannot allow.
    OpenObject,
    /// A `true` subschema: any value at all.
    AnyValue,
    /// `oneOf` beside `anyOf` on one schema (`oneOf` is sent as `anyOf`).
    OneOfWithAnyOf,
}

impl fmt::Display for Unsupported {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Recursive => "recursive",
            Self::UnresolvedRef => "an unresolved $ref",
            Self::OpenObject => "an open object",
            Self::AnyValue => "an any-value subschema",
            Self::OneOfWithAnyOf => "oneOf beside anyOf",
        })
    }
}

/// The schema structured outputs receive for `schema` (see the module docs).
pub fn output_schema(schema: &Value) -> OutputSchema {
    match check_shapes(schema).and_then(|()| check_refs(schema)) {
        Ok(()) => OutputSchema::Constrained(rewrite(schema)),
        Err(reason) => OutputSchema::Unconstrained(reason),
    }
}

/// Fails on the shapes no rewrite can express.
fn check_shapes(node: &Value) -> Result<(), Unsupported> {
    let map = match node {
        Value::Object(map) => map,
        Value::Bool(true) => return Err(Unsupported::AnyValue),
        _ => return Ok(()),
    };
    let open = map
        .get("additionalProperties")
        .is_some_and(|extra| extra != &Value::Bool(false));
    if open || map.contains_key("patternProperties") {
        return Err(Unsupported::OpenObject);
    }
    if map.contains_key("oneOf") && map.contains_key("anyOf") {
        return Err(Unsupported::OneOfWithAnyOf);
    }
    subschemas(map, true).into_iter().try_for_each(check_shapes)
}

/// The subschemas directly under `map`; `with_defs` includes definitions.
fn subschemas(map: &Map<String, Value>, with_defs: bool) -> Vec<&Value> {
    let mut found = Vec::new();
    for (key, value) in map {
        let key = key.as_str();
        match value {
            _ if !with_defs && (key == "$defs" || key == "definitions") => {}
            Value::Object(children) if SCHEMA_MAPS.contains(&key) => {
                found.extend(children.values());
            }
            Value::Array(children) if SCHEMA_LISTS.contains(&key) || key == "items" => {
                found.extend(children);
            }
            child if SCHEMA_VALUES.contains(&key) => found.push(child),
            _ => {}
        }
    }
    found
}

/// `node` in the structured-outputs subset (after the checks passed).
fn rewrite(node: &Value) -> Value {
    match node {
        Value::Object(map) => Value::Object(rewrite_object(map)),
        other => other.clone(),
    }
}

fn rewrite_object(map: &Map<String, Value>) -> Map<String, Value> {
    let mut out: Map<String, Value> = map
        .iter()
        .filter_map(|(key, value)| rewrite_entry(key, value))
        .collect();
    if is_object_schema(&out) {
        out.insert("additionalProperties".to_owned(), Value::Bool(false));
    }
    out
}

/// One keyword rewritten: kept, renamed, recursed into, or dropped.
fn rewrite_entry(key: &str, value: &Value) -> Option<(String, Value)> {
    let rewritten = match key {
        _ if STRIPPED.contains(&key) => return None,
        "format" => {
            let supported = value
                .as_str()
                .is_some_and(|format| FORMATS.contains(&format));
            return supported.then(|| (key.to_owned(), value.clone()));
        }
        "oneOf" => return Some(("anyOf".to_owned(), rewrite_each(value))),
        _ if SCHEMA_LISTS.contains(&key) => rewrite_each(value),
        _ if SCHEMA_MAPS.contains(&key) => rewrite_values(value),
        "items" if value.is_array() => rewrite_each(value),
        _ if SCHEMA_VALUES.contains(&key) => rewrite(value),
        _ => value.clone(),
    };
    Some((key.to_owned(), rewritten))
}

fn rewrite_each(list: &Value) -> Value {
    match list {
        Value::Array(items) => Value::Array(items.iter().map(rewrite).collect()),
        other => other.clone(),
    }
}

fn rewrite_values(map: &Value) -> Value {
    match map {
        Value::Object(entries) => Value::Object(
            entries
                .iter()
                .map(|(name, schema)| (name.clone(), rewrite(schema)))
                .collect(),
        ),
        other => other.clone(),
    }
}

/// Whether `map` describes an object (by `type`, or by having `properties`).
fn is_object_schema(map: &Map<String, Value>) -> bool {
    let typed = match map.get("type") {
        Some(Value::String(kind)) => kind == "object",
        Some(Value::Array(kinds)) => kinds.iter().any(|kind| kind == "object"),
        _ => false,
    };
    typed || map.contains_key("properties")
}

#[cfg(any(test, feature = "test-support"))]
pub use self::audit::constrained_schema;

#[cfg(any(test, feature = "test-support"))]
mod audit;

#[cfg(test)]
mod schema_tests;
