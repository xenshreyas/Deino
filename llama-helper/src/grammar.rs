//! Constrained decoding: a JSON schema becomes a GBNF grammar sampler.
//!
//! `grammar` requests carry a JSON schema. llama.cpp converts it to a GBNF
//! grammar ([`schema_to_gbnf`]) and builds a sampler from it
//! ([`grammar_sampler`]), which `sampling::build` puts first in the chain.

use anyhow::{Context, Result};
use llama_cpp_2::model::LlamaModel;
use llama_cpp_2::sampling::LlamaSampler;

/// The grammar rule generation starts from.
pub const GRAMMAR_ROOT: &str = "root";

/// The GBNF grammar for a JSON schema.
pub fn schema_to_gbnf(schema: &serde_json::Value) -> Result<String> {
    llama_cpp_2::json_schema_to_grammar(&schema.to_string()).context("Invalid JSON schema")
}

/// A sampler that only allows output matching `schema`.
pub fn grammar_sampler(model: &LlamaModel, schema: &serde_json::Value) -> Result<LlamaSampler> {
    let gbnf = schema_to_gbnf(schema)?;
    LlamaSampler::grammar(model, &gbnf, GRAMMAR_ROOT).context("Invalid grammar")
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::schema_to_gbnf;

    #[test]
    fn converts_an_object_schema_to_a_grammar_with_a_root_rule() {
        let schema = json!({
            "type": "object",
            "properties": {
                "title": {"type": "string"},
                "done": {"type": "boolean"}
            },
            "required": ["title", "done"]
        });
        let gbnf = schema_to_gbnf(&schema).unwrap();
        assert!(gbnf.contains("root ::="), "{gbnf}");
        assert!(gbnf.contains("title"), "{gbnf}");
    }

    #[test]
    fn rejects_a_schema_it_cannot_convert() {
        let result = schema_to_gbnf(&json!({"type": "no-such-type"}));
        assert!(result.is_err());
    }
}
