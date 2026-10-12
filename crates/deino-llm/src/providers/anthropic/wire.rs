//! Anthropic Messages API wire types (`POST /v1/messages`, streaming).
//!
//! Requests borrow from the [`CompletionRequest`], so building one copies no
//! prompt text. Responses are the server-sent events of a streamed message.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{CompletionRequest, OutputFormat, Part, Role};

/// What the forced tool of a JSON answer tells the model.
const JSON_TOOL_DESCRIPTION: &str =
    "Record your answer. Call this exactly once with the complete answer as its input.";

/// The request body.
#[derive(Debug, Serialize)]
pub(super) struct MessagesRequest<'a> {
    model: &'a str,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<&'a str>,
    messages: Vec<Message<'a>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<Tool<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<ToolChoice<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    stream: bool,
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

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ToolChoice<'a> {
    Tool { name: &'a str },
}

impl<'a> MessagesRequest<'a> {
    /// The streamed request for `request`. A JSON answer uses the tool trick:
    /// one tool whose input schema is the answer's schema, forced.
    pub(super) fn new(request: &'a CompletionRequest) -> Self {
        let (tools, tool_choice) = match &request.output {
            OutputFormat::Json(json) => (
                vec![Tool {
                    name: &json.name,
                    description: JSON_TOOL_DESCRIPTION,
                    input_schema: &json.schema,
                }],
                Some(ToolChoice::Tool { name: &json.name }),
            ),
            OutputFormat::Text => (request.tools.iter().map(tool).collect(), None),
        };
        Self {
            model: &request.model,
            max_tokens: request.max_tokens,
            system: request.system.as_deref(),
            messages: request.messages.iter().map(message).collect(),
            tools,
            tool_choice,
            temperature: request.temperature,
            stream: true,
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
