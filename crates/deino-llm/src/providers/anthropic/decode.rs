//! Anthropic's streamed events → [`CompletionEvent`]s.
//!
//! Text deltas pass through; thinking (which Sonnet and Haiku 5.5 do by
//! default, before the answer) is dropped. A tool call's input arrives as JSON
//! fragments (`input_json_delta`) and is emitted as one `ToolUse` when its
//! block stops. In JSON-answer mode the structured output arrives as text
//! deltas, which are the answer; no tool is offered, so tool blocks are
//! ignored. A JSON answer cut off at `max_tokens` is not JSON, so it ends the
//! stream with [`LlmError::Truncated`] (after its usage) instead of reaching
//! the parser. Usage and the stop reason are emitted once, at `message_stop`.

use std::collections::BTreeMap;

use serde_json::Value;

use super::wire::{ContentBlock, Delta, ErrorBody, MessageDelta, StreamEvent, WireUsage};
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
    /// A decoder; `json_answer` when the request asked for a JSON answer.
    pub(super) fn new(json_answer: bool) -> Self {
        Self {
            json_answer,
            ..Self::default()
        }
    }

    fn on(&mut self, event: StreamEvent) -> Result<Vec<CompletionEvent>, LlmError> {
        Ok(match event {
            StreamEvent::MessageStart { message } => {
                self.count(message.usage);
                Vec::new()
            }
            StreamEvent::ContentBlockStart {
                index,
                content_block,
            } => self.start_block(index, content_block),
            StreamEvent::ContentBlockDelta { index, delta } => self.delta(index, delta),
            StreamEvent::ContentBlockStop { index } => self.stop_block(index)?,
            StreamEvent::MessageDelta { delta, usage } => self.message_delta(&delta, usage),
            StreamEvent::MessageStop => self.close()?,
            StreamEvent::Error { error } => return Err(stream_error(error)),
            StreamEvent::Other => Vec::new(),
        })
    }

    /// Records the stop reason and final usage; a cut-off JSON answer
    /// reports its usage now, since its stream ends in an error.
    fn message_delta(
        &mut self,
        delta: &MessageDelta,
        usage: Option<WireUsage>,
    ) -> Vec<CompletionEvent> {
        self.stop = Some(stop_reason(delta.stop_reason.as_deref(), self.json_answer));
        if let Some(usage) = usage {
            self.count(usage);
        }
        if self.cut_off() {
            vec![CompletionEvent::Usage(self.usage)]
        } else {
            Vec::new()
        }
    }

    /// Whether this is a JSON answer that `max_tokens` cut off.
    fn cut_off(&self) -> bool {
        self.json_answer && self.stop == Some(StopReason::MaxTokens)
    }

    /// The closing usage and stop, or [`LlmError::Truncated`] for a JSON
    /// answer cut off at `max_tokens` (its usage went out with `message_delta`).
    fn close(&self) -> Result<Vec<CompletionEvent>, LlmError> {
        if self.cut_off() {
            log::warn!(target: "llm", "anthropic: a JSON answer hit max_tokens after {} output tokens", self.usage.output_tokens);
            return Err(LlmError::Truncated);
        }
        Ok(vec![
            CompletionEvent::Usage(self.usage),
            CompletionEvent::Stop(self.stop.unwrap_or(StopReason::EndTurn)),
        ])
    }

    fn count(&mut self, usage: WireUsage) {
        self.usage = self.usage.merge(Usage {
            input_tokens: usage.input_tokens,
            output_tokens: usage.output_tokens,
        });
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
            Delta::Text { text } => vec![CompletionEvent::Text(text)],
            Delta::InputJson { partial_json } => {
                if let Some(tool) = self.tools.get_mut(&index) {
                    tool.input.push_str(&partial_json);
                }
                Vec::new()
            }
            Delta::Other => Vec::new(),
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
