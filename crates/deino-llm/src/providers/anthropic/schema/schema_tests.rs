//! The structured-outputs rewrite: what changes, what is kept, and which
//! schemas fall back to an unconstrained answer.

use schemars::JsonSchema;
use serde_json::{Value, json};

use super::*;
use crate::json_schema_for;

fn constrained(schema: &Value) -> Value {
    match output_schema(schema) {
        OutputSchema::Constrained(sent) => sent,
        OutputSchema::Unconstrained(reason) => panic!("unexpectedly {reason}"),
    }
}

fn unsupported(schema: &Value) -> Unsupported {
    match output_schema(schema) {
        OutputSchema::Unconstrained(reason) => reason,
        OutputSchema::Constrained(sent) => panic!("unexpectedly constrained: {sent}"),
    }
}

#[test]
fn every_object_is_closed_at_any_depth() {
    let schema = json!({
        "type": "object",
        "properties": {
            "owner": {
                "type": ["object", "null"],
                "properties": { "name": { "type": "string" } }
            },
            "tasks": {
                "type": "array",
                "items": { "type": "object", "properties": { "text": { "type": "string" } } }
            }
        },
        "required": ["tasks"]
    });

    let sent = constrained(&schema);

    assert_eq!(sent["additionalProperties"], false);
    assert_eq!(sent["properties"]["owner"]["additionalProperties"], false);
    assert_eq!(
        sent["properties"]["tasks"]["items"]["additionalProperties"],
        false
    );
    assert_eq!(sent["required"], json!(["tasks"]));
    assert!(
        sent["properties"]["tasks"]
            .get("additionalProperties")
            .is_none()
    );
}

#[test]
fn definitions_are_closed_and_references_kept() {
    let schema = json!({
        "type": "object",
        "properties": { "item": { "$ref": "#/$defs/Item" } },
        "$defs": {
            "Item": { "type": "object", "properties": { "n": { "type": "integer", "minimum": 0 } } }
        },
        "definitions": { "Other": { "properties": { "flag": { "type": "boolean" } } } }
    });

    let sent = constrained(&schema);

    assert_eq!(
        sent["properties"]["item"],
        json!({ "$ref": "#/$defs/Item" })
    );
    assert_eq!(sent["$defs"]["Item"]["additionalProperties"], false);
    assert_eq!(
        sent["$defs"]["Item"]["properties"]["n"],
        json!({ "type": "integer" })
    );
    assert_eq!(sent["definitions"]["Other"]["additionalProperties"], false);
}

#[test]
fn array_size_rules_are_stripped_and_tuple_items_rewritten() {
    let schema = json!({
        "type": "array",
        "items": [{ "type": "object" }, { "type": "string", "maxLength": 9 }],
        "minItems": 1,
        "maxItems": 3,
        "uniqueItems": true
    });

    let sent = constrained(&schema);

    assert_eq!(
        sent,
        json!({
            "type": "array",
            "items": [{ "type": "object", "additionalProperties": false }, { "type": "string" }]
        })
    );
}

#[test]
fn any_of_branches_are_rewritten_and_one_of_becomes_any_of() {
    let schema = json!({
        "type": "object",
        "properties": {
            "either": { "anyOf": [{ "type": "object", "properties": {} }, { "type": "null" }] },
            "kind": { "oneOf": [{ "const": "a", "description": "A." }, { "const": "b" }] }
        }
    });

    let sent = constrained(&schema);

    assert_eq!(
        sent["properties"]["either"]["anyOf"][0]["additionalProperties"],
        false
    );
    assert_eq!(
        sent["properties"]["kind"],
        json!({ "anyOf": [{ "const": "a", "description": "A." }, { "const": "b" }] })
    );
}

#[test]
fn refused_keywords_and_formats_are_dropped_and_the_rest_kept() {
    let schema = json!({
        "type": "object",
        "properties": {
            "count": { "type": "integer", "format": "uint32", "minimum": 0, "maximum": 9, "multipleOf": 1 },
            "ratio": { "type": "number", "format": "double", "exclusiveMinimum": 0 },
            "name": { "type": "string", "minLength": 1, "maxLength": 80, "pattern": "^[a-z]+$" },
            "when": { "type": "string", "format": "date-time", "description": "When." },
            "level": { "enum": ["low", "high", null], "default": "low" }
        }
    });

    let sent = constrained(&schema);

    let properties = &sent["properties"];
    assert_eq!(properties["count"], json!({ "type": "integer" }));
    assert_eq!(properties["ratio"], json!({ "type": "number" }));
    assert_eq!(
        properties["name"],
        json!({ "type": "string", "pattern": "^[a-z]+$" })
    );
    assert_eq!(properties["when"]["format"], "date-time");
    assert_eq!(properties["level"], schema["properties"]["level"]);
}

#[test]
fn property_names_and_values_that_look_like_keywords_are_untouched() {
    let schema = json!({
        "type": "object",
        "properties": {
            "format": { "type": "string" },
            "minimum": { "type": "number" },
            "config": { "const": { "maxItems": 2 } }
        }
    });

    let sent = constrained(&schema);

    assert_eq!(sent["properties"]["format"], json!({ "type": "string" }));
    assert_eq!(sent["properties"]["minimum"], json!({ "type": "number" }));
    assert_eq!(
        sent["properties"]["config"],
        json!({ "const": { "maxItems": 2 } })
    );
}

#[test]
fn recursion_is_detected_through_definitions_and_the_root() {
    let through_definitions = json!({
        "$ref": "#/$defs/Node",
        "$defs": {
            "Node": { "type": "object", "properties": { "children": { "type": "array", "items": { "$ref": "#/$defs/Child" } } } },
            "Child": { "anyOf": [{ "$ref": "#/$defs/Node" }, { "type": "null" }] }
        }
    });
    let through_the_root = json!({
        "type": "object",
        "properties": { "next": { "$ref": "#" } }
    });

    assert_eq!(unsupported(&through_definitions), Unsupported::Recursive);
    assert_eq!(unsupported(&through_the_root), Unsupported::Recursive);
}

#[test]
fn shared_definitions_without_a_cycle_are_not_recursive() {
    let schema = json!({
        "type": "object",
        "properties": { "a": { "$ref": "#/$defs/Leaf" }, "b": { "$ref": "#/$defs/Pair" } },
        "$defs": {
            "Pair": { "type": "array", "items": { "$ref": "#/$defs/Leaf" } },
            "Leaf": { "type": "string" }
        }
    });

    assert!(matches!(
        output_schema(&schema),
        OutputSchema::Constrained(_)
    ));
}

/// A note-tree shape: what a recursive Deino type would look like.
#[allow(dead_code, reason = "only its schema is used")]
#[derive(JsonSchema)]
struct Outline {
    title: String,
    children: Vec<Outline>,
}

#[test]
fn a_recursive_rust_type_falls_back_to_an_unconstrained_answer() {
    // `json_schema_for` drops the definitions a recursive type still needs.
    let reason = unsupported(&json_schema_for::<Outline>());

    assert!(
        matches!(reason, Unsupported::Recursive | Unsupported::UnresolvedRef),
        "{reason}"
    );
}

#[test]
fn unresolvable_references_open_maps_and_any_values_are_unsupported() {
    let dangling = json!({ "properties": { "a": { "$ref": "#/$defs/Gone" } } });
    let pointer = json!({ "properties": { "a": { "$ref": "#/properties/b" } } });
    let map = json!({ "type": "object", "additionalProperties": { "type": "string" } });
    let open = json!({ "type": "object", "additionalProperties": true });
    let patterned = json!({ "type": "object", "patternProperties": { "^x": {} } });
    let any = json!({ "type": "object", "properties": { "value": true } });
    let both = json!({ "anyOf": [{ "type": "string" }], "oneOf": [{ "type": "null" }] });

    assert_eq!(unsupported(&dangling), Unsupported::UnresolvedRef);
    assert_eq!(unsupported(&pointer), Unsupported::UnresolvedRef);
    assert_eq!(unsupported(&map), Unsupported::OpenObject);
    assert_eq!(unsupported(&open), Unsupported::OpenObject);
    assert_eq!(unsupported(&patterned), Unsupported::OpenObject);
    assert_eq!(unsupported(&any), Unsupported::AnyValue);
    assert_eq!(unsupported(&both), Unsupported::OneOfWithAnyOf);
}

#[test]
fn a_closed_object_stays_closed() {
    let schema = json!({ "type": "object", "properties": {}, "additionalProperties": false });

    assert_eq!(constrained(&schema), schema);
}

#[test]
fn the_audit_rejects_what_the_api_would_refuse() {
    let unconstrained = json!({ "type": "object", "additionalProperties": { "type": "string" } });

    assert!(constrained_schema(&unconstrained).is_err());
    assert!(constrained_schema(&json!({ "type": "object" })).is_ok());
}

#[test]
fn this_crate_s_structured_answers_are_constrained() {
    let schemas = [
        json_schema_for::<crate::extraction::ActionItemsAnswer>(),
        json_schema_for::<crate::brief::BriefAnswer>(),
        json_schema_for::<crate::followup::FollowUpDecision>(),
        json_schema_for::<crate::followup::FollowUpDraftAnswer>(),
    ];

    for schema in &schemas {
        if let Err(problem) = constrained_schema(schema) {
            panic!("{problem}: {schema}");
        }
    }
}
