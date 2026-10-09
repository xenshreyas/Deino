//! The provider-agnostic request model: what callers ask for ([`Prompt`])
//! and what a provider receives ([`CompletionRequest`]).
//!
//! Tool inputs and JSON schemas are `serde_json::Value`: they are opaque
//! payloads between a typed tool or `structured::<T>()` and the provider, and
//! each is parsed into its Rust type at that edge (§3A typing).

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{ProviderId, Route};

/// Who said a message.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    /// The user, or the app speaking for the user (tool results included).
    User,
    /// The model.
    Assistant,
}

/// One turn of a conversation.
#[derive(Clone, Debug, PartialEq)]
pub struct Message {
    /// Who said it.
    pub role: Role,
    /// What was said, in order.
    pub parts: Vec<Part>,
}

/// A piece of a message.
#[derive(Clone, Debug, PartialEq)]
pub enum Part {
    /// Plain text.
    Text(String),
    /// The model asked to call a tool (assistant messages only).
    ToolUse(ToolCall),
    /// The answer to a tool call (user messages only).
    ToolResult(ToolResult),
}

/// A tool call the model asked for.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolCall {
    /// The provider's ID for this call, echoed back in the [`ToolResult`].
    pub id: String,
    /// The tool's name.
    pub name: String,
    /// The tool input, validated against the tool's schema by the tool.
    pub input: Value,
}

/// The app's answer to a [`ToolCall`].
#[derive(Clone, Debug, PartialEq)]
pub struct ToolResult {
    /// The [`ToolCall::id`] this answers.
    pub call_id: String,
    /// What the tool returned, or why it failed.
    pub content: String,
    /// Whether the tool failed; the model may retry or give up.
    pub is_error: bool,
}

impl Message {
    /// A user message with one text part.
    pub fn user(text: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            parts: vec![Part::Text(text.into())],
        }
    }

    /// An assistant message with one text part.
    pub fn assistant(text: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            parts: vec![Part::Text(text.into())],
        }
    }

    /// The message's text parts, joined.
    pub fn text(&self) -> String {
        self.parts
            .iter()
            .filter_map(|part| match part {
                Part::Text(text) => Some(text.as_str()),
                Part::ToolUse(_) | Part::ToolResult(_) => None,
            })
            .collect()
    }
}

/// A tool the model may call, as providers receive it.
#[derive(Clone, Debug, PartialEq)]
pub struct ToolDefinition {
    /// A unique name, `snake_case`.
    pub name: String,
    /// When and why the model should call it.
    pub description: String,
    /// A draft-07 JSON schema of the input, subschemas inlined.
    pub input_schema: Value,
}

/// The shape of the answer.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum OutputFormat {
    /// Free text (Markdown).
    #[default]
    Text,
    /// One JSON value matching a schema. Each provider enforces it its own
    /// way: Anthropic with a forced tool, OpenAI with `response_format`, the
    /// built-in model with a grammar. The JSON arrives as `Text` events.
    Json(JsonOutput),
}

/// A JSON answer's schema.
#[derive(Clone, Debug, PartialEq)]
pub struct JsonOutput {
    /// A short `snake_case` name for the schema (some providers require one).
    pub name: String,
    /// A draft-07 JSON schema, subschemas inlined.
    pub schema: Value,
}

/// A provider and model, chosen by a route or pinned by the caller.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
pub struct Target {
    /// Who serves the call.
    pub provider: ProviderId,
    /// The provider's model ID.
    pub model: String,
}

/// What a caller asks the gateway: the conversation and its limits.
///
/// The provider and model come from the route unless [`Prompt::on`] pins them.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Prompt {
    /// Instructions for the model.
    pub system: Option<String>,
    /// The conversation, oldest first, ending with a user message.
    pub messages: Vec<Message>,
    /// The most tokens the answer may use; the gateway's default otherwise.
    pub max_tokens: Option<u32>,
    /// Sampling temperature; the provider's default otherwise.
    pub temperature: Option<f32>,
    /// A provider and model that bypass the route's settings.
    pub target: Option<Target>,
}

impl Prompt {
    /// A one-message conversation.
    pub fn new(user: impl Into<String>) -> Self {
        Self {
            messages: vec![Message::user(user)],
            ..Self::default()
        }
    }

    /// Sets the system prompt.
    #[must_use]
    pub fn system(mut self, system: impl Into<String>) -> Self {
        self.system = Some(system.into());
        self
    }

    /// Caps the answer's length.
    #[must_use]
    pub fn max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Serves this prompt from `target`, whatever the route's settings say.
    #[must_use]
    pub fn on(mut self, target: Target) -> Self {
        self.target = Some(target);
        self
    }
}

/// Exactly what one provider call sends.
#[derive(Clone, Debug, PartialEq)]
pub struct CompletionRequest {
    /// The route the call serves (for providers keyed by route, and logs).
    pub route: Route,
    /// The provider's model ID.
    pub model: String,
    /// Instructions for the model.
    pub system: Option<String>,
    /// The conversation, oldest first.
    pub messages: Vec<Message>,
    /// Tools the model may call.
    pub tools: Vec<ToolDefinition>,
    /// The shape of the answer.
    pub output: OutputFormat,
    /// The most tokens the answer may use.
    pub max_tokens: u32,
    /// Sampling temperature; the provider's default when `None`.
    pub temperature: Option<f32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_text_skips_tool_parts() {
        let message = Message {
            role: Role::Assistant,
            parts: vec![
                Part::Text("Looking it up. ".into()),
                Part::ToolUse(ToolCall {
                    id: "call-1".into(),
                    name: "search".into(),
                    input: Value::Null,
                }),
                Part::Text("Done.".into()),
            ],
        };

        assert_eq!(message.text(), "Looking it up. Done.");
    }
}
