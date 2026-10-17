//! Anthropic Messages API wire types (`POST /v1/messages`, streaming).
//!
//! Requests borrow from the [`CompletionRequest`], so building one copies no
//! prompt text. Responses are the server-sent events of a streamed message.
//!
//! A JSON answer is a structured output: `output_config.format` carries the
//! answer's schema, rewritten to the subset the API accepts
//! ([`output_schema`]), and the JSON arrives as ordinary text. Claude
//! 5.1/5.5-generation models refuse the older tool trick (a forced
//! `tool_choice`), so no request sends one. No request sends `temperature`
//! either: Haiku 5.5 refuses any value but its default and Sonnet 5.5 asks
//! callers to drop non-default sampling, so every Claude model samples at
//! its own default. Every request's `max_tokens` is at least
//! [`ANTHROPIC_MIN_TOKENS`], since 5.5 models think before they answer.

use std::borrow::Cow;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::schema::{OutputSchema, output_schema};
use crate::{CompletionRequest, JsonOutput, OutputFormat, Part, Role};

/// Every request's `max_tokens`, at least. Sonnet and Haiku 5.5 think
/// adaptively by default and thinking counts against `max_tokens`, so a
/// route cap sized for the answer alone (a 40-token title, a JSON answer)
/// could be spent on thinking and leave the answer empty or cut off. The
/// request streams, so a large cap is no timeout risk, and it is a ceiling,
/// not a spend: the prompts still ask for short answers. 16 000 cut the
/// notes of a 72-minute meeting off mid-JSON (Oct 2026); 48 000 fits within
/// every 5.5 model's output limit. Other providers keep the route's cap.
pub(super) const ANTHROPIC_MIN_TOKENS: u32 = 48_000;

/// What a schema the API cannot constrain asks for in the system prompt.
const JSON_INSTRUCTION: &str = "Reply with only one JSON value that matches this JSON schema, with no Markdown fence and no other text.";

/// The request body.
#[derive(Debug, Serialize)]
pub(super) struct MessagesRequest<'a> {
    model: &'a str,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<Cow<'a, str>>,
    messages: Vec<Message<'a>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<Tool<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    output_config: Option<OutputConfig>,
    stream: bool,
}

/// `output_config`: the answer's format.
#[derive(Debug, Serialize)]
struct OutputConfig {
    format: JsonSchemaFormat,
}

/// `{"type": "json_schema", "schema": …}`.
#[derive(Debug, Serialize)]
struct JsonSchemaFormat {
    #[serde(rename = "type")]
    kind: &'static str,
    schema: Value,
}

#[derive(Debug, Serialize)]
struct Message<'a> {
    role: &'static str,
    content: Vec<Block<'a>>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Block<'a> {
    Text {
        text: &'a str,
    },
    ToolUse {
        id: &'a str,
        name: &'a str,
        input: &'a Value,
    },
    ToolResult {
        tool_use_id: &'a str,
        content: &'a str,
        is_error: bool,
    },
    Image {
        source: ImageSource<'a>,
    },
}

/// A base64 picture (`{"type": "base64", "media_type", "data"}`).
#[derive(Debug, Serialize)]
struct ImageSource<'a> {
    #[serde(rename = "type")]
    kind: &'static str,
    media_type: &'a str,
    data: &'a str,
}

#[derive(Debug, Serialize)]
struct Tool<'a> {
    name: &'a str,
    description: &'a str,
    input_schema: &'a Value,
}

impl<'a> MessagesRequest<'a> {
    /// The streamed request for `request` (see the module docs).
    pub(super) fn new(request: &'a CompletionRequest) -> Self {
        let mut body = Self {
            model: &request.model,
            max_tokens: request.max_tokens.max(ANTHROPIC_MIN_TOKENS),
            system: request.system.as_deref().map(Cow::Borrowed),
            messages: request.messages.iter().map(message).collect(),
            tools: Vec::new(),
            output_config: None,
            stream: true,
        };
        match &request.output {
            OutputFormat::Json(json) => body.answer_in_json(json),
            OutputFormat::Text => body.tools = request.tools.iter().map(tool).collect(),
        }
        body
    }

    /// A JSON answer: a structured output when the schema allows one, the
    /// schema in the system prompt otherwise; no tools either way.
    fn answer_in_json(&mut self, json: &JsonOutput) {
        match output_schema(&json.schema) {
            OutputSchema::Constrained(schema) => {
                let format = JsonSchemaFormat {
                    kind: "json_schema",
                    schema,
                };
                self.output_config = Some(OutputConfig { format });
            }
            OutputSchema::Unconstrained(reason) => {
                log::info!(target: "llm", "anthropic: the {} schema is {reason}; asking for JSON in the prompt", json.name);
                let instruction = format!("{JSON_INSTRUCTION}\n\n{}", json.schema);
                self.system = Some(Cow::Owned(match self.system.take() {
                    Some(system) => format!("{system}\n\n{instruction}"),
                    None => instruction,
                }));
            }
        }
    }
}

fn tool(definition: &crate::ToolDefinition) -> Tool<'_> {
    Tool {
        name: &definition.name,
        description: &definition.description,
        input_schema: &definition.input_schema,
    }
}

fn message(message: &crate::Message) -> Message<'_> {
    let role = match message.role {
        Role::User => "user",
        Role::Assistant => "assistant",
    };
    let content = message
        .parts
        .iter()
        .map(|part| match part {
            Part::Text(text) => Block::Text { text },
            Part::ToolUse(call) => Block::ToolUse {
                id: &call.id,
                name: &call.name,
                input: &call.input,
            },
            Part::ToolResult(result) => Block::ToolResult {
                tool_use_id: &result.call_id,
                content: &result.content,
                is_error: result.is_error,
            },
            Part::Image(image) => Block::Image {
                source: ImageSource {
                    kind: "base64",
                    media_type: &image.media_type,
                    data: &image.base64,
                },
            },
        })
        .collect();
    Message { role, content }
}

/// One server-sent event of a streamed message.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(super) enum StreamEvent {
    MessageStart {
        message: MessageStart,
    },
    ContentBlockStart {
        index: usize,
        content_block: ContentBlock,
    },
    ContentBlockDelta {
        index: usize,
        delta: Delta,
    },
    ContentBlockStop {
        index: usize,
    },
    MessageDelta {
        delta: MessageDelta,
        usage: Option<WireUsage>,
    },
    MessageStop,
    Error {
        error: ErrorBody,
    },
    /// `ping` and event types added after this was written.
    #[serde(other)]
    Other,
}

#[derive(Debug, Deserialize)]
pub(super) struct MessageStart {
    pub usage: WireUsage,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(super) enum ContentBlock {
    ToolUse {
        id: String,
        name: String,
    },
    /// Text blocks (their text arrives as deltas), thinking and redacted thinking.
    #[serde(other)]
    Other,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(super) enum Delta {
    #[serde(rename = "text_delta")]
    Text { text: String },
    #[serde(rename = "input_json_delta")]
    InputJson { partial_json: String },
    /// Thinking and signature deltas: reasoning is never shown or stored.
    #[serde(other)]
    Other,
}

#[derive(Debug, Deserialize)]
pub(super) struct MessageDelta {
    pub stop_reason: Option<String>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize)]
pub(super) struct WireUsage {
    #[serde(default)]
    pub input_tokens: u32,
    #[serde(default)]
    pub output_tokens: u32,
}

#[derive(Debug, Deserialize)]
pub(super) struct ErrorBody {
    #[serde(rename = "type")]
    pub kind: String,
    pub message: String,
}

/// `GET /v1/models`.
#[derive(Debug, Deserialize)]
pub(super) struct ModelList {
    pub data: Vec<ModelEntry>,
}

#[derive(Debug, Deserialize)]
pub(super) struct ModelEntry {
    pub id: String,
    pub display_name: Option<String>,
}
