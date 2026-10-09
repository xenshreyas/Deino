//! What a provider streams back, and the collected answer.
//!
//! Every provider turns its wire format into the same four events (the zed
//! event model, reimplemented). A well-formed stream ends with exactly one
//! [`CompletionEvent::Stop`]; [`Completion::collect`] rejects one that does not.
//! Model reasoning (Anthropic thinking blocks, OpenAI `reasoning_content`) is
//! never emitted: it is neither shown nor stored.

use futures_util::StreamExt;
use futures_util::stream::BoxStream;
use serde::Serialize;

use crate::{LlmError, ToolCall};

/// One step of a streamed answer.
#[derive(Clone, Debug, PartialEq)]
pub enum CompletionEvent {
    /// The next piece of the answer's text.
    Text(String),
    /// The model asks to call a tool; the input is complete.
    ToolUse(ToolCall),
    /// Token counts so far (providers report them cumulatively).
    Usage(Usage),
    /// The answer is complete.
    Stop(StopReason),
}

/// Token counts of one call.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Usage {
    /// Tokens the provider read: system prompt, messages, tool definitions.
    pub input_tokens: u32,
    /// Tokens the provider wrote.
    pub output_tokens: u32,
}

impl Usage {
    /// Folds a later report in. Providers report counts cumulatively and
    /// some split them across events, so each field keeps its largest value.
    #[must_use]
    pub fn merge(self, later: Self) -> Self {
        Self {
            input_tokens: self.input_tokens.max(later.input_tokens),
            output_tokens: self.output_tokens.max(later.output_tokens),
        }
    }
}

/// Why the model stopped.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    /// The answer is finished.
    EndTurn,
    /// The answer hit `max_tokens` and is cut short.
    MaxTokens,
    /// The model wants its tool calls answered before it continues.
    ToolUse,
    /// A stop sequence ended the answer.
    StopSequence,
    /// The provider declined to answer (safety).
    Refusal,
}

/// A provider's answer as it arrives.
pub type EventStream = BoxStream<'static, Result<CompletionEvent, LlmError>>;

/// A whole answer, collected from an [`EventStream`].
#[derive(Clone, Debug, PartialEq)]
pub struct Completion {
    /// The answer's text.
    pub text: String,
    /// The tool calls the model asked for, in order.
    pub tool_calls: Vec<ToolCall>,
    /// Token counts.
    pub usage: Usage,
    /// Why the model stopped.
    pub stop: StopReason,
}

impl Completion {
    /// Reads a stream to its end.
    ///
    /// Fails with the stream's first error, or with
    /// [`LlmError::Truncated`] when the stream ends without a stop event.
    pub async fn collect(mut stream: EventStream) -> Result<Self, LlmError> {
        let mut text = String::new();
        let mut tool_calls = Vec::new();
        let mut usage = Usage::default();
        let mut stop = None;
        while let Some(event) = stream.next().await {
            match event? {
                CompletionEvent::Text(piece) => text.push_str(&piece),
                CompletionEvent::ToolUse(call) => tool_calls.push(call),
                CompletionEvent::Usage(report) => usage = usage.merge(report),
                CompletionEvent::Stop(reason) => stop = Some(reason),
            }
        }
        let stop = stop.ok_or(LlmError::Truncated)?;
        Ok(Self {
            text,
            tool_calls,
            usage,
            stop,
        })
    }
}

#[cfg(test)]
mod tests {
    use futures_util::stream;
    use serde_json::json;

    use super::*;

    fn events(events: Vec<Result<CompletionEvent, LlmError>>) -> EventStream {
        stream::iter(events).boxed()
    }

    #[tokio::test]
    async fn collect_joins_text_keeps_tool_calls_and_merges_usage() {
        let call = ToolCall {
            id: "call-1".into(),
            name: "search".into(),
            input: json!({ "query": "pilot" }),
        };
        let stream = events(vec![
            Ok(CompletionEvent::Usage(Usage {
                input_tokens: 12,
                output_tokens: 1,
            })),
            Ok(CompletionEvent::Text("Hello ".into())),
            Ok(CompletionEvent::Text("world".into())),
            Ok(CompletionEvent::ToolUse(call.clone())),
            Ok(CompletionEvent::Usage(Usage {
                input_tokens: 0,
                output_tokens: 7,
            })),
            Ok(CompletionEvent::Stop(StopReason::ToolUse)),
        ]);

        let completion = Completion::collect(stream).await.unwrap();

        assert_eq!(completion.text, "Hello world");
        assert_eq!(completion.tool_calls, [call]);
        assert_eq!(
            completion.usage,
            Usage {
                input_tokens: 12,
                output_tokens: 7
            }
        );
        assert_eq!(completion.stop, StopReason::ToolUse);
    }

    #[tokio::test]
    async fn a_stream_without_a_stop_is_truncated() {
        let stream = events(vec![Ok(CompletionEvent::Text("Partial".into()))]);

        let error = Completion::collect(stream).await.unwrap_err();

        assert!(matches!(error, LlmError::Truncated));
    }

    #[tokio::test]
    async fn the_first_error_ends_collection() {
        let stream = events(vec![
            Ok(CompletionEvent::Text("Partial".into())),
            Err(LlmError::Cancelled),
            Ok(CompletionEvent::Stop(StopReason::EndTurn)),
        ]);

        let error = Completion::collect(stream).await.unwrap_err();

        assert!(matches!(error, LlmError::Cancelled));
    }
}
