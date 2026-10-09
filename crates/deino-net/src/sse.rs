//! Server-Sent Events: the `text/event-stream` format LLM providers stream
//! tokens in, decoded from response body bytes.
//!
//! [`SseDecoder`] follows the WHATWG parsing rules that providers rely on:
//! `\n` or `\r\n` line ends, `data:` lines joined with `\n`, `event:` and
//! `id:` fields, `:` comments (keep-alives), and an event per blank line.

use std::collections::VecDeque;

use bytes::Bytes;
use futures_util::StreamExt;
use futures_util::stream::{self, BoxStream};

use crate::{BodyStream, NetError};

/// A stream of decoded events.
pub type SseStream = BoxStream<'static, Result<SseEvent, NetError>>;

/// One Server-Sent Event.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SseEvent {
    /// The `event:` type, if the server named one.
    pub event: Option<String>,
    /// The `data:` lines, joined with `\n`.
    pub data: String,
    /// The last `id:` seen.
    pub id: Option<String>,
}

/// Turns body chunks into events; chunks may split lines anywhere.
#[derive(Debug, Default)]
pub struct SseDecoder {
    buffer: Vec<u8>,
    event: Option<String>,
    data: Option<String>,
    id: Option<String>,
}

impl SseDecoder {
    /// Feeds `chunk` and returns every event it completed.
    pub fn push(&mut self, chunk: &[u8]) -> Vec<SseEvent> {
        self.buffer.extend_from_slice(chunk);
        let mut events = Vec::new();
        while let Some(end) = self.buffer.iter().position(|&byte| byte == b'\n') {
            let line: Vec<u8> = self.buffer.drain(..=end).collect();
            let line = line.strip_suffix(b"\n").unwrap_or(&line);
            let line = line.strip_suffix(b"\r").unwrap_or(line);
            if let Some(event) = self.line(&String::from_utf8_lossy(line)) {
                events.push(event);
            }
        }
        events
    }

    /// Handles one line; a blank line completes the pending event.
    fn line(&mut self, line: &str) -> Option<SseEvent> {
        if line.is_empty() {
            return self.dispatch();
        }
        if line.starts_with(':') {
            return None;
        }
        let (field, value) = line.split_once(':').unwrap_or((line, ""));
        let value = value.strip_prefix(' ').unwrap_or(value);
        match field {
            "data" => {
                let data = self.data.get_or_insert_with(String::new);
                if !data.is_empty() {
                    data.push('\n');
                }
                data.push_str(value);
            }
            "event" => self.event = Some(value.to_owned()),
            "id" => self.id = Some(value.to_owned()),
            _ => {}
        }
        None
    }

    /// The pending event, if it has data; the event type resets either way.
    fn dispatch(&mut self) -> Option<SseEvent> {
        let event = self.event.take();
        let data = self.data.take()?;
        Some(SseEvent {
            event,
            data,
            id: self.id.clone(),
        })
    }
}

/// Decodes `body` into events. A transport error ends the stream after it.
pub fn events(body: BodyStream) -> SseStream {
    let state = (body, SseDecoder::default(), VecDeque::new());
    stream::unfold(state, |(mut body, mut decoder, mut ready)| async move {
        loop {
            if let Some(event) = ready.pop_front() {
                return Some((Ok(event), (body, decoder, ready)));
            }
            match body.next().await? {
                Ok(chunk) => ready.extend(decoder.push(&chunk)),
                Err(error) => {
                    let ended: BodyStream = stream::empty::<Result<Bytes, NetError>>().boxed();
                    return Some((Err(error), (ended, decoder, ready)));
                }
            }
        }
    })
    .boxed()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode(chunks: &[&str]) -> Vec<SseEvent> {
        let mut decoder = SseDecoder::default();
        chunks
            .iter()
            .flat_map(|chunk| decoder.push(chunk.as_bytes()))
            .collect()
    }

    #[test]
    fn events_survive_chunks_split_mid_line_and_crlf_line_ends() {
        let events = decode(&["event: delta\r\nda", "ta: {\"t\":1}\r\n\r", "\n"]);

        assert_eq!(
            events,
            [SseEvent {
                event: Some("delta".into()),
                data: "{\"t\":1}".into(),
                id: None,
            }]
        );
    }

    #[test]
    fn data_lines_join_comments_are_skipped_and_empty_events_dropped() {
        let events = decode(&[": keep-alive\n\nevent: ping\n\ndata: a\ndata:b\nid: 7\n\n"]);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "a\nb");
        assert_eq!(events[0].event, None);
        assert_eq!(events[0].id.as_deref(), Some("7"));
    }

    #[test]
    fn an_unfinished_event_is_not_emitted() {
        assert!(decode(&["data: partial\n"]).is_empty());
    }
}
