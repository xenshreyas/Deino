//! Shared plumbing from a streamed HTTP body to an [`EventStream`].
//!
//! Each HTTP provider supplies an [`SseDecoder`] (its wire events →
//! [`CompletionEvent`]s); [`decode_sse`] drives it over the body. The first
//! error ends the stream, as the provider contract requires.

use std::collections::VecDeque;

use futures_util::StreamExt;
use futures_util::stream::{self, BoxStream};
use serde::de::DeserializeOwned;

use crate::{
    BodyStream, CompletionEvent, EventStream, HttpError, LlmError, ProviderId, SseEvent, sse,
};

/// Turns one provider's server-sent events into completion events.
pub(crate) trait SseDecoder: Send + 'static {
    /// The events one server-sent event completes.
    fn decode(&mut self, event: SseEvent) -> Result<Vec<CompletionEvent>, LlmError>;

    /// The events left when the body ends (for wire formats whose last event
    /// is implicit).
    fn finish(&mut self) -> Result<Vec<CompletionEvent>, LlmError> {
        Ok(Vec::new())
    }
}

/// Parses a stream event's JSON, reporting only the error class on failure.
pub(crate) fn parse_event<T: DeserializeOwned>(
    provider: ProviderId,
    data: &str,
) -> Result<T, LlmError> {
    serde_json::from_str(data).map_err(|error| LlmError::Malformed {
        provider,
        detail: format!("unreadable stream event ({})", category(&error)),
    })
}

/// A serde error's category, never its message (which quotes the input).
pub(crate) fn category(error: &serde_json::Error) -> &'static str {
    match error.classify() {
        serde_json::error::Category::Io => "io",
        serde_json::error::Category::Syntax => "syntax",
        serde_json::error::Category::Data => "data",
        serde_json::error::Category::Eof => "eof",
    }
}

/// Decodes `body` as server-sent events with `decoder`.
pub(crate) fn decode_sse<D: SseDecoder>(
    provider: ProviderId,
    body: BodyStream,
    decoder: D,
) -> EventStream {
    let state = Decoding {
        provider,
        events: sse::events(body),
        decoder,
        ready: VecDeque::new(),
        done: false,
    };
    stream::unfold(state, Decoding::next).boxed()
}

/// The unfold state of [`decode_sse`].
struct Decoding<D> {
    provider: ProviderId,
    events: BoxStream<'static, Result<SseEvent, HttpError>>,
    decoder: D,
    ready: VecDeque<CompletionEvent>,
    done: bool,
}

impl<D: SseDecoder> Decoding<D> {
    async fn next(mut self) -> Option<(Result<CompletionEvent, LlmError>, Self)> {
        loop {
            if let Some(event) = self.ready.pop_front() {
                return Some((Ok(event), self));
            }
            if self.done {
                return None;
            }
            let decoded = match self.events.next().await {
                Some(Ok(event)) => self.decoder.decode(event),
                Some(Err(source)) => Err(LlmError::Network {
                    provider: self.provider,
                    source,
                }),
                None => {
                    self.done = true;
                    self.decoder.finish()
                }
            };
            match decoded {
                Ok(events) => self.ready.extend(events),
                Err(error) => {
                    self.done = true;
                    return Some((Err(error), self));
                }
            }
        }
    }
}
