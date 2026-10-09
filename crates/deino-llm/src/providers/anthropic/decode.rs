//! Anthropic's streamed events → [`CompletionEvent`]s.
//!
//! Text deltas pass through. A tool call's input arrives as JSON fragments
//! (`input_json_delta`) and is emitted as one `ToolUse` when its block stops.
//! In JSON-answer mode the forced tool's fragments *are* the answer, so they
//! pass through as `Text` instead. Usage and the stop reason are emitted once,
//! at `message_stop`.

use std::collections::BTreeMap;

use serde_json::Value;

use super::wire::{ContentBlock, Delta, ErrorBody, StreamEvent};
use crate::providers::status;
use crate::providers::stream::{SseDecoder, parse_event};
use crate::{CompletionEvent, LlmError, ProviderId, SseEvent, StopReason, ToolCall, Usage};

const PROVIDER: ProviderId = ProviderId::Anthropic;

/// The decoding state of one streamed message.
#[derive(Debug, Default)]
pub(super) struct Decoder {
    json_answer: bool,
    tools: BTreeMap<usize, PendingTool>,
    usage: Usage,
    stop: Option<StopReason>,
}

#[derive(Debug)]
struct PendingTool {
    id: String,
    name: String,
    input: String,
}

impl Decoder {
    /// A decoder; `json_answer` when the request forced the JSON tool.
    pub(super) fn new(json_answer: bool) -> Self {
        Self {
            json_answer,
            ..Self::default()
        }
    }

    fn on(&mut self, event: StreamEvent) -> Result<Vec<CompletionEvent>, LlmError> {
        Ok(match event {
            StreamEvent::MessageStart { message } => self.count(message.usage),
            StreamEvent::ContentBlockStart {
                index,
                content_block,
            } => self.start_block(index, content_block),
            StreamEvent::ContentBlockDelta { index, delta } => self.delta(index, delta),
            StreamEvent::ContentBlockStop { index } => self.stop_block(index)?,
            StreamEvent::MessageDelta { delta, usage } => {
                self.stop = Some(stop_reason(delta.stop_reason.as_deref(), self.json_answer));
                usage.map(|usage| self.count(usage)).unwrap_or_default()
            }
            StreamEvent::MessageStop => vec![
                CompletionEvent::Usage(self.usage),
                CompletionEvent::Stop(self.stop.unwrap_or(StopReason::EndTurn)),
            ],
            StreamEvent::Error { error } => return Err(stream_error(error)),
            StreamEvent::Other => Vec::new(),
        })
    }

    fn count(&mut self, usage: super::wire::WireUsage) -> Vec<CompletionEvent> {
        self.usage = self.usage.merge(Usage {
            input_tokens: usage.input_tokens,
            output_tokens: usage.output_tokens,
        });
        Vec::new()
    }

    fn start_block(&mut self, index: usize, block: ContentBlock) -> Vec<CompletionEvent> {
        if let ContentBlock::ToolUse { id, name } = block
            && !self.json_answer
        {
            let input = String::new();
            self.tools.insert(index, PendingTool { id, name, input });
        }
        Vec::new()
    }

    fn delta(&mut self, index: usize, delta: Delta) -> Vec<CompletionEvent> {
        match delta {
            Delta::Text { text } if !self.json_answer => vec![CompletionEvent::Text(text)],
            Delta::InputJson { partial_json } if self.json_answer => {
                vec![CompletionEvent::Text(partial_json)]
            }
            Delta::InputJson { partial_json } => {
                if let Some(tool) = self.tools.get_mut(&index) {
                    tool.input.push_str(&partial_json);
                }
                Vec::new()
            }
            Delta::Text { .. } | Delta::Other => Vec::new(),
        }
    }

    fn stop_block(&mut self, index: usize) -> Result<Vec<CompletionEvent>, LlmError> {
        let Some(tool) = self.tools.remove(&index) else {
            return Ok(Vec::new());
        };
        let input = if tool.input.trim().is_empty() {
            Value::Object(serde_json::Map::new())
        } else {
            serde_json::from_str(&tool.input).map_err(|_| LlmError::Malformed {
                provider: PROVIDER,
                detail: format!("the input of tool call {} is not JSON", tool.id),
            })?
        };
        Ok(vec![CompletionEvent::ToolUse(ToolCall {
            id: tool.id,
            name: tool.name,
            input,
        })])
    }
}

impl SseDecoder for Decoder {
    fn decode(&mut self, event: SseEvent) -> Result<Vec<CompletionEvent>, LlmError> {
        let event = parse_event(PROVIDER, &event.data)?;
        self.on(event)
    }
}

fn stop_reason(reason: Option<&str>, json_answer: bool) -> StopReason {
    match reason {
        Some("max_tokens") => StopReason::MaxTokens,
        Some("tool_use") if !json_answer => StopReason::ToolUse,
        Some("stop_sequence") => StopReason::StopSequence,
        Some("refusal") => StopReason::Refusal,
        _ => StopReason::EndTurn,
    }
}

fn stream_error(error: ErrorBody) -> LlmError {
    match error.kind.as_str() {
        "overloaded_error" => LlmError::Overloaded {
            provider: PROVIDER,
            retry_after: None,
        },
        "rate_limit_error" => LlmError::RateLimited {
            provider: PROVIDER,
            retry_after: None,
        },
        "authentication_error" | "permission_error" => LlmError::Auth(PROVIDER),
        "invalid_request_error" => status::classify(PROVIDER, 400, None, error.message),
        _ => status::classify(PROVIDER, 0, None, error.message),
    }
}
