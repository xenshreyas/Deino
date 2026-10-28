//! Structured output: `Gateway::structured::<T>()` returns a typed value.
//!
//! The schema is draft-07 from `schemars`, with subschemas inlined and the
//! `$schema`/`definitions` keys removed so every provider accepts it (the
//! zed tool-schema technique). Each provider enforces it its own way (see
//! [`OutputFormat::Json`]). The answer is parsed with serde; when that fails,
//! the gateway asks once more, showing the model its answer and the parse
//! error, and fails with [`LlmError::InvalidOutput`] if the repair fails too.
//! `T` must be a JSON object (a struct), and must not be recursive:
//! Anthropic's structured outputs refuse recursive schemas (such a `T` is
//! asked for as prompt-only JSON instead) and drop constraints such as
//! `maxLength` or `minimum`, so the serde parse here and each caller's own
//! validation are the checks that always run. A JSON answer cut off at `max_tokens` fails as
//! [`LlmError::Truncated`] on Anthropic rather than reaching the repair.

use schemars::generate::SchemaSettings;
use schemars::{JsonSchema, Schema};
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::gateway::Shape;
use crate::providers::stream::category;
use crate::{Gateway, JsonOutput, LlmError, Message, OutputFormat, Prompt, Route};

/// What the repair turn tells the model.
const REPAIR_INSTRUCTION: &str = "Your answer could not be used. Reply again with only the corrected JSON, matching the schema exactly.";

/// The draft-07 schema of `T`, every subschema inlined.
pub fn json_schema_for<T: JsonSchema>() -> Value {
    let generator = SchemaSettings::draft07()
        .with(|settings| settings.inline_subschemas = true)
        .into_generator();
    let mut schema: Schema = generator.into_root_schema_for::<T>();
    if let Some(object) = schema.as_object_mut() {
        object.remove("$schema");
        object.remove("definitions");
        object.remove("$defs");
    }
    schema.to_value()
}

impl Gateway {
    /// A `T` for `route`, from a schema-constrained answer with one repair
    /// retry (see the module docs).
    pub async fn structured<T: JsonSchema + DeserializeOwned>(
        &self,
        route: Route,
        prompt: Prompt,
    ) -> Result<T, LlmError> {
        let output = JsonOutput {
            name: schema_name::<T>(),
            schema: json_schema_for::<T>(),
        };
        self.structured_as(route, prompt, output).await
    }

    /// A `T` for `route` under a schema built at run time, for answers whose
    /// shape the user defines (AI-06's insight fields). `output` constrains
    /// the provider; `T` is the typed wire value the answer parses into, which
    /// the caller still validates. The same repair retry as
    /// [`Gateway::structured`].
    pub async fn structured_as<T: DeserializeOwned>(
        &self,
        route: Route,
        prompt: Prompt,
        output: JsonOutput,
    ) -> Result<T, LlmError> {
        let shape = Shape {
            output: OutputFormat::Json(output),
            tools: Vec::new(),
        };
        let first = self
            .complete_shaped(route, prompt.clone(), shape.clone())
            .await?;
        let error = match parse::<T>(&first.text) {
            Ok(value) => return Ok(value),
            Err(error) => error,
        };
        log::info!(target: "llm", "{route}: structured answer did not parse ({}, stop {:?}, {} chars); repairing once", category(&error), first.stop, first.text.len());
        let repair = repair_prompt(prompt, &first.text, &error);
        let second = self.complete_shaped(route, repair, shape).await?;
        parse::<T>(&second.text).map_err(|error| {
            log::warn!(target: "llm", "{route}: repaired answer did not parse either ({}, stop {:?}, {} chars)", category(&error), second.stop, second.text.len());
            invalid_output(&error)
        })
    }
}

/// The error of an answer that did not parse, without its text.
fn invalid_output(error: &serde_json::Error) -> LlmError {
    LlmError::InvalidOutput {
        detail: format!(
            "{} error at line {} column {}",
            category(error),
            error.line(),
            error.column()
        ),
    }
}

/// A provider-safe name for `T`'s schema: `snake_case` letters, digits and `_`.
fn schema_name<T: JsonSchema>() -> String {
    let mut name = String::new();
    for (index, character) in T::schema_name().chars().enumerate() {
        if character.is_ascii_uppercase() && index > 0 {
            name.push('_');
        }
        if character.is_ascii_alphanumeric() {
            name.push(character.to_ascii_lowercase());
        }
    }
    if name.is_empty() {
        "answer".to_owned()
    } else {
        name
    }
}

/// Parses an answer, tolerating a Markdown fence or prose around the JSON.
fn parse<T: DeserializeOwned>(text: &str) -> Result<T, serde_json::Error> {
    serde_json::from_str(json_span(text))
}

/// The answer from its first `{` to its last `}`, or all of it.
fn json_span(text: &str) -> &str {
    match (text.find('{'), text.rfind('}')) {
        (Some(start), Some(end)) if start < end => text.get(start..=end).unwrap_or(text),
        _ => text.trim(),
    }
}

fn repair_prompt(mut prompt: Prompt, answer: &str, error: &serde_json::Error) -> Prompt {
    prompt.messages.push(Message::assistant(answer));
    prompt.messages.push(Message::user(format!(
        "{REPAIR_INSTRUCTION}\nParse error: {error}"
    )));
    prompt
}

#[cfg(test)]
mod structured_tests;
