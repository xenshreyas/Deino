//! OpenAI-compatible streamed chunks → [`CompletionEvent`]s.
//!
//! Content deltas pass through as text. Tool calls arrive as fragments keyed
//! by index (the first carries the ID and name, later ones append argument
//! JSON) and are emitted whole at the end. The end is `data: [DONE]`, or the
//! end of the body for servers that omit it; the usage chunk, when asked
//! for, arrives after `finish_reason`, so the stop waits for the end.

use std::collections::BTreeMap;

use serde_json::Value;

use super::response::{Chunk, ToolCallDelta, WireUsage};
use crate::providers::status;
use crate::providers::stream::{SseDecoder, parse_event};
use crate::{CompletionEvent, LlmError, ProviderId, SseEvent, StopReason, ToolCall, Usage};

/// The decoding state of one streamed completion.
#[derive(Debug)]
pub(super) struct Decoder {
    provider: ProviderId,
    tools: BTreeMap<usize, PendingTool>,
    usage: Usage,
    stop: Option<StopReason>,
    finished: bool,
}

#[derive(Debug, Default)]
struct PendingTool {
    id: String,
    name: String,
    arguments: String,
}

impl Decoder {
    pub(super) fn new(provider: ProviderId) -> Self {
        Self {
            provider,
            tools: BTreeMap::new(),
            usage: Usage::default(),
            stop: None,
            finished: false,
        }
    }

    fn on(&mut self, chunk: Chunk) -> Result<Vec<CompletionEvent>, LlmError> {
        if let Some(error) = chunk.error {
            let message = status::provider_message(error.to_string().as_bytes(), 0);
            return Err(status::classify(self.provider, 0, None, message));
        }
        if let Some(usage) = chunk.usage {
            self.count(usage);
        }
        let mut events = Vec::new();
        for choice in chunk.choices {
            if let Some(text) = choice.delta.content.filter(|text| !text.is_empty()) {
                events.push(CompletionEvent::Text(text));
            }
            for delta in choice.delta.tool_calls {
                self.add_tool_fragment(delta);
            }
            if let Some(reason) = choice.finish_reason {
                self.stop = Some(stop_reason(&reason));
            }
        }
        Ok(events)
    }

    fn count(&mut self, usage: WireUsage) {
        self.usage = self.usage.merge(Usage {
            input_tokens: usage.prompt_tokens,
            output_tokens: usage.completion_tokens,
        });
    }

    fn add_tool_fragment(&mut self, delta: ToolCallDelta) {
        let tool = self.tools.entry(delta.index).or_default();
        if let Some(id) = delta.id {
            tool.id = id;
        }
        let Some(function) = delta.function else {
            return;
        };
        if let Some(name) = function.name {
            tool.name.push_str(&name);
        }
        if let Some(arguments) = function.arguments {
            tool.arguments.push_str(&arguments);
        }
    }

    /// The tool calls, usage and stop, once. Nothing when the stream never
    /// finished, so the collector reports it as truncated.
    fn end(&mut self) -> Result<Vec<CompletionEvent>, LlmError> {
        let Some(stop) = self.stop.filter(|_| !self.finished) else {
            return Ok(Vec::new());
        };
        self.finished = true;
        let mut events = Vec::new();
        for tool in std::mem::take(&mut self.tools).into_values() {
            events.push(CompletionEvent::ToolUse(self.tool_call(tool)?));
        }
        events.push(CompletionEvent::Usage(self.usage));
        events.push(CompletionEvent::Stop(stop));
        Ok(events)
    }

    fn tool_call(&self, tool: PendingTool) -> Result<ToolCall, LlmError> {
        let input = if tool.arguments.trim().is_empty() {
            Value::Object(serde_json::Map::new())
        } else {
            serde_json::from_str(&tool.arguments).map_err(|_| LlmError::Malformed {
                provider: self.provider,
                detail: format!("the arguments of tool call {} are not JSON", tool.id),
            })?
        };
        Ok(ToolCall {
            id: tool.id,
            name: tool.name,
            input,
        })
    }
}

impl SseDecoder for Decoder {
    fn decode(&mut self, event: SseEvent) -> Result<Vec<CompletionEvent>, LlmError> {
        if event.data.trim() == "[DONE]" {
            return self.end();
        }
        let chunk = parse_event(self.provider, &event.data)?;
        self.on(chunk)
    }

    fn finish(&mut self) -> Result<Vec<CompletionEvent>, LlmError> {
        self.end()
    }
}

fn stop_reason(reason: &str) -> StopReason {
    match reason {
        "length" => StopReason::MaxTokens,
        "tool_calls" | "function_call" => StopReason::ToolUse,
        "content_filter" => StopReason::Refusal,
        _ => StopReason::EndTurn,
    }
}
