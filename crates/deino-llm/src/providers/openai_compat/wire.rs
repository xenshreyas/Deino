//! OpenAI Chat Completions request types (`POST {base}/chat/completions`,
//! streamed), shared by OpenAI, Groq, OpenRouter, custom endpoints and
//! Ollama. The response side is `response.rs`.

use std::borrow::Cow;

use serde::Serialize;
use serde_json::Value;

use crate::{CompletionRequest, JsonOutput, Message, OutputFormat, Part, Role, SamplingOverrides};

/// The request body.
#[derive(Debug, Serialize)]
pub(super) struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<ChatMessage<'a>>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream_options: Option<StreamOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<Tool<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<ResponseFormat<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reasoning_effort: Option<&'static str>,
}

/// The optional fields a request may carry, which some servers reject.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Extras {
    /// Ask for a final usage chunk (`stream_options.include_usage`).
    pub usage_chunk: bool,
    /// Enforce a JSON answer with `response_format: json_schema`.
    pub response_format: bool,
    /// `reasoning_effort`, when the flavor sets one.
    pub reasoning_effort: Option<&'static str>,
}

#[derive(Debug, Serialize)]
struct StreamOptions {
    include_usage: bool,
}

#[derive(Debug, Serialize)]
struct ChatMessage<'a> {
    role: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<Cow<'a, str>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tool_calls: Vec<WireToolCall<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<&'a str>,
}

#[derive(Debug, Serialize)]
struct WireToolCall<'a> {
    id: &'a str,
    #[serde(rename = "type")]
    kind: &'static str,
    function: FunctionCall<'a>,
}

#[derive(Debug, Serialize)]
struct FunctionCall<'a> {
    name: &'a str,
    arguments: String,
}

#[derive(Debug, Serialize)]
struct Tool<'a> {
    #[serde(rename = "type")]
    kind: &'static str,
    function: FunctionDefinition<'a>,
}

#[derive(Debug, Serialize)]
struct FunctionDefinition<'a> {
    name: &'a str,
    description: &'a str,
    parameters: &'a Value,
}

#[derive(Debug, Serialize)]
struct ResponseFormat<'a> {
    #[serde(rename = "type")]
    kind: &'static str,
    json_schema: JsonSchemaFormat<'a>,
}

#[derive(Debug, Serialize)]
struct JsonSchemaFormat<'a> {
    name: &'a str,
    schema: &'a Value,
    // Strict mode requires every property to be required and closed; Deino's
    // schemas allow optional fields, so the parse-and-repair step enforces them.
    strict: bool,
}

impl<'a> ChatRequest<'a> {
    /// The streamed request for `request` with `extras` and the endpoint's
    /// sampling overrides. Without `response_format`, a JSON answer's schema
    /// goes into the system prompt instead (the prompt-only fallback).
    pub(super) fn new(
        request: &'a CompletionRequest,
        extras: Extras,
        sampling: SamplingOverrides,
    ) -> Self {
        let json = match &request.output {
            OutputFormat::Json(json) => Some(json),
            OutputFormat::Text => None,
        };
        let response_format = json.filter(|_| extras.response_format).map(response_format);
        let schema_in_prompt = json.filter(|_| response_format.is_none());
        Self {
            model: &request.model,
            messages: messages(request, schema_in_prompt),
            stream: true,
            stream_options: extras.usage_chunk.then_some(StreamOptions {
                include_usage: true,
            }),
            max_tokens: sampling.max_tokens,
            temperature: sampling.temperature.or(request.temperature),
            top_p: sampling.top_p,
            tools: request.tools.iter().map(tool).collect(),
            response_format,
            reasoning_effort: extras.reasoning_effort,
        }
    }
}

fn response_format(json: &JsonOutput) -> ResponseFormat<'_> {
    ResponseFormat {
        kind: "json_schema",
        json_schema: JsonSchemaFormat {
            name: &json.name,
            schema: &json.schema,
            strict: false,
        },
    }
}

fn tool(definition: &crate::ToolDefinition) -> Tool<'_> {
    Tool {
        kind: "function",
        function: FunctionDefinition {
            name: &definition.name,
            description: &definition.description,
            parameters: &definition.input_schema,
        },
    }
}

fn messages<'a>(
    request: &'a CompletionRequest,
    schema_in_prompt: Option<&JsonOutput>,
) -> Vec<ChatMessage<'a>> {
    let system = system_prompt(request.system.as_deref(), schema_in_prompt);
    let mut wire: Vec<ChatMessage<'a>> = system
        .into_iter()
        .map(|text| text_message("system", text))
        .collect();
    for message in &request.messages {
        wire.extend(convert(message));
    }
    wire
}

fn system_prompt<'a>(system: Option<&'a str>, schema: Option<&JsonOutput>) -> Option<Cow<'a, str>> {
    let Some(schema) = schema else {
        return system.map(Cow::Borrowed);
    };
    let instructions = format!(
        "Answer with only a JSON value that matches this JSON schema, and nothing else:\n{}",
        schema.schema
    );
    Some(match system {
        Some(system) => Cow::Owned(format!("{system}\n\n{instructions}")),
        None => Cow::Owned(instructions),
    })
}

fn text_message<'a>(role: &'static str, text: Cow<'a, str>) -> ChatMessage<'a> {
    ChatMessage {
        role,
        content: Some(text),
        tool_calls: Vec::new(),
        tool_call_id: None,
    }
}

/// One message becomes a user or assistant message plus one `tool` message
/// per tool result (OpenAI keeps tool results out of user messages).
fn convert(message: &Message) -> Vec<ChatMessage<'_>> {
    let text = message.text();
    let mut wire = Vec::new();
    let tool_calls: Vec<WireToolCall<'_>> = message
        .parts
        .iter()
        .filter_map(|part| match part {
            Part::ToolUse(call) => Some(WireToolCall {
                id: &call.id,
                kind: "function",
                function: FunctionCall {
                    name: &call.name,
                    arguments: call.input.to_string(),
                },
            }),
            Part::Text(_) | Part::ToolResult(_) => None,
        })
        .collect();
    let role = match message.role {
        Role::User => "user",
        Role::Assistant => "assistant",
    };
    if !text.is_empty() || !tool_calls.is_empty() {
        let content = (!text.is_empty()).then_some(Cow::Owned(text));
        wire.push(ChatMessage {
            role,
            content,
            tool_calls,
            tool_call_id: None,
        });
    }
    wire.extend(message.parts.iter().filter_map(|part| match part {
        Part::ToolResult(result) => Some(ChatMessage {
            role: "tool",
            content: Some(Cow::Borrowed(result.content.as_str())),
            tool_calls: Vec::new(),
            tool_call_id: Some(&result.call_id),
        }),
        Part::Text(_) | Part::ToolUse(_) => None,
    }));
    wire
}
