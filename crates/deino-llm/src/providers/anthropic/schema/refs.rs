//! `$ref` checks: every reference resolves, and none is recursive.
//!
//! The graph's nodes are the root (`#`) and the root's `$defs` and
//! `definitions`; an edge is a `$ref` inside a node (outside nested
//! definitions). A cycle anywhere is a recursive schema, which structured
//! outputs refuse. `schemars` keeps a recursive type as a `$ref` even when it
//! inlines subschemas, and `structured.rs` drops the definitions, so a
//! recursive Deino type shows up here as an unresolved `$ref`; both fall back
//! to an unconstrained answer.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use super::{Unsupported, subschemas};

/// The `$ref` of the root schema, and its node in the graph.
const ROOT: &str = "#";

/// Fails when a `$ref` does not resolve or the references form a cycle.
pub(super) fn check_refs(schema: &Value) -> Result<(), Unsupported> {
    let definitions = definitions(schema);
    let mut graph = BTreeMap::new();
    graph.insert(ROOT, targets(schema, &definitions)?);
    for (name, definition) in &definitions {
        graph.insert(*name, targets(definition, &definitions)?);
    }
    let mut done = BTreeSet::new();
    let recursive = graph
        .keys()
        .any(|start| cycles_from(&graph, start, &mut Vec::new(), &mut done));
    if recursive {
        Err(Unsupported::Recursive)
    } else {
        Ok(())
    }
}

/// The root's `$defs` and `definitions`, by name.
fn definitions(schema: &Value) -> BTreeMap<&str, &Value> {
    ["$defs", "definitions"]
        .iter()
        .filter_map(|key| schema.get(key).and_then(Value::as_object))
        .flat_map(|map| map.iter().map(|(name, value)| (name.as_str(), value)))
        .collect()
}

/// The graph nodes `node` refers to.
fn targets<'a>(
    node: &'a Value,
    definitions: &BTreeMap<&str, &Value>,
) -> Result<Vec<&'a str>, Unsupported> {
    let mut refs = Vec::new();
    collect_refs(node, &mut refs);
    refs.into_iter()
        .map(|target| resolve(target, definitions))
        .collect()
}

fn collect_refs<'a>(node: &'a Value, refs: &mut Vec<&'a str>) {
    let Value::Object(map) = node else {
        return;
    };
    if let Some(Value::String(target)) = map.get("$ref") {
        refs.push(target);
    }
    for child in subschemas(map, false) {
        collect_refs(child, refs);
    }
}

/// The graph node a `$ref` names: the root, or a root definition.
fn resolve<'a>(
    target: &'a str,
    definitions: &BTreeMap<&str, &Value>,
) -> Result<&'a str, Unsupported> {
    if target == ROOT {
        return Ok(ROOT);
    }
    let name = target
        .strip_prefix("#/$defs/")
        .or_else(|| target.strip_prefix("#/definitions/"))
        .ok_or(Unsupported::UnresolvedRef)?;
    if definitions.contains_key(name) {
        Ok(name)
    } else {
        Err(Unsupported::UnresolvedRef)
    }
}

/// Whether a walk from `node` comes back to a node on `path`.
fn cycles_from<'a>(
    graph: &BTreeMap<&'a str, Vec<&'a str>>,
    node: &'a str,
    path: &mut Vec<&'a str>,
    done: &mut BTreeSet<&'a str>,
) -> bool {
    if path.contains(&node) {
        return true;
    }
    if done.contains(node) {
        return false;
    }
    path.push(node);
    let found = graph
        .get(node)
        .into_iter()
        .flatten()
        .any(|next| cycles_from(graph, next, path, done));
    path.pop();
    done.insert(node);
    found
}
