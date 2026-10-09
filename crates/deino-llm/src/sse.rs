//! Server-sent events: the one parser every HTTP provider shares.
//!
//! [`SseParser`] follows the WHATWG event-stream rules: lines end with LF,
//! CRLF or CR (a CRLF split across chunks counts once), `:` starts a
//! comment, `data` lines join with `\n`, and a blank line dispatches. It works
//! on bytes, so a UTF-8 character split across network chunks is decoded
//! whole. At the end of the body a pending event is dispatched even without
//! the final blank line, which some OpenAI-compatible servers omit.
//!
//! [`events`] turns a response body into a stream of [`SseEvent`]s.

use std::collections::VecDeque;
use std::mem;

use futures_util::StreamExt;
use futures_util::stream::{self, BoxStream};

use crate::{BodyStream, HttpError};

/// One dispatched event.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SseEvent {
    /// The `event:` field, if the server named the event.
    pub event: Option<String>,
    /// The `data:` lines, joined with `\n`.
    pub data: String,
}

/// An incremental event-stream parser. Feed it chunks in order.
#[derive(Debug, Default)]
pub struct SseParser {
    line: Vec<u8>,
    after_cr: bool,
    event: Option<String>,
    data: String,
    has_data: bool,
}

impl SseParser {
    /// Parses the next chunk and returns the events it completed.
    pub fn push(&mut self, chunk: &[u8]) -> Vec<SseEvent> {
        let mut events = Vec::new();
        for &byte in chunk {
            let after_cr = mem::take(&mut self.after_cr);
            match byte {
                b'\n' if after_cr => {}
                b'\n' => self.end_line(&mut events),
                b'\r' => {
                    self.end_line(&mut events);
                    self.after_cr = true;
                }
                _ => self.line.push(byte),
            }
        }
        events
    }

    /// Ends the stream: processes an unterminated last line and dispatches
    /// the pending event, if any.
    pub fn finish(&mut self) -> Option<SseEvent> {
        if !self.line.is_empty() {
            let line = mem::take(&mut self.line);
            self.field(&String::from_utf8_lossy(&line));
        }
        self.dispatch()
    }

    fn end_line(&mut self, events: &mut Vec<SseEvent>) {
        let line = mem::take(&mut self.line);
        if line.is_empty() {
            events.extend(self.dispatch());
        } else {
            self.field(&String::from_utf8_lossy(&line));
        }
    }

    fn field(&mut self, line: &str) {
        if line.starts_with(':') {
            return;
        }
        let (name, value) = line.split_once(':').unwrap_or((line, ""));
        let value = value.strip_prefix(' ').unwrap_or(value);
        match name {
            "event" => self.event = Some(value.to_owned()),
            "data" => {
                if self.has_data {
                    self.data.push('\n');
                }
                self.data.push_str(value);
                self.has_data = true;
            }
            // `id`, `retry` and unknown fields do not matter to LLM streams.
            _ => {}
        }
    }

    fn dispatch(&mut self) -> Option<SseEvent> {
        let event = self.event.take();
        if !mem::take(&mut self.has_data) {
            return None;
        }
        Some(SseEvent {
            event,
            data: mem::take(&mut self.data),
        })
    }
}

/// The events of a response body, in order. A transport error ends the stream.
pub fn events(body: BodyStream) -> BoxStream<'static, Result<SseEvent, HttpError>> {
    let state = Reader {
        body,
        parser: SseParser::default(),
        ready: VecDeque::new(),
        done: false,
    };
    stream::unfold(state, Reader::next).boxed()
}

/// The unfold state of [`events`].
struct Reader {
    body: BodyStream,
    parser: SseParser,
    ready: VecDeque<SseEvent>,
    done: bool,
}

impl Reader {
    async fn next(mut self) -> Option<(Result<SseEvent, HttpError>, Self)> {
        loop {
            if let Some(event) = self.ready.pop_front() {
                return Some((Ok(event), self));
            }
            if self.done {
                return None;
            }
            match self.body.next().await {
                Some(Ok(chunk)) => self.ready.extend(self.parser.push(&chunk)),
                Some(Err(error)) => {
                    self.done = true;
                    return Some((Err(error), self));
                }
                None => {
                    self.done = true;
                    self.ready.extend(self.parser.finish());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    fn parse_all(chunks: &[&[u8]]) -> Vec<SseEvent> {
        let mut parser = SseParser::default();
        let mut events: Vec<SseEvent> =
            chunks.iter().flat_map(|chunk| parser.push(chunk)).collect();
        events.extend(parser.finish());
        events
    }

    fn event(name: Option<&str>, data: &str) -> SseEvent {
        SseEvent {
            event: name.map(str::to_owned),
            data: data.to_owned(),
        }
    }

    #[test]
    fn named_events_multi_line_data_and_comments() {
        let body =
            b": keep-alive\nevent: message_start\ndata: {\"a\":1}\n\ndata: one\ndata: two\n\n";

        assert_eq!(
            parse_all(&[body]),
            [
                event(Some("message_start"), "{\"a\":1}"),
                event(None, "one\ntwo")
            ]
        );
    }

    #[test]
    fn crlf_split_across_chunks_counts_as_one_line_end() {
        let events = parse_all(&[b"data: a\r", b"\n\r", b"\ndata: b\r\r"]);

        assert_eq!(events, [event(None, "a"), event(None, "b")]);
    }

    #[test]
    fn a_character_split_across_chunks_is_decoded_whole() {
        let bytes = "data: café\n\n".as_bytes();
        let (head, tail) = bytes.split_at(10);

        assert_eq!(parse_all(&[head, tail]), [event(None, "café")]);
    }

    #[test]
    fn a_pending_event_is_dispatched_at_the_end_of_the_body() {
        assert_eq!(parse_all(&[b"data: [DONE]"]), [event(None, "[DONE]")]);
    }

    #[test]
    fn fields_without_a_space_and_blank_dispatches_without_data() {
        assert_eq!(
            parse_all(&[b"event: ping\n\ndata:x\n\n"]),
            [event(None, "x")]
        );
    }

    fn arbitrary_event() -> impl Strategy<Value = SseEvent> {
        (
            proptest::option::of("[a-z_]{1,12}"),
            proptest::collection::vec("[^\r\n]{0,24}", 1..4),
        )
            .prop_map(|(name, lines)| SseEvent {
                event: name,
                data: lines.join("\n"),
            })
    }

    fn encode(events: &[SseEvent], line_end: &str) -> Vec<u8> {
        let mut body = String::new();
        for event in events {
            if let Some(name) = &event.event {
                body.push_str("event: ");
                body.push_str(name);
                body.push_str(line_end);
            }
            for line in event.data.split('\n') {
                body.push_str("data: ");
                body.push_str(line);
                body.push_str(line_end);
            }
            body.push_str(line_end);
        }
        body.into_bytes()
    }

    proptest! {
        #[test]
        fn chunk_boundaries_never_change_the_events(
            events in proptest::collection::vec(arbitrary_event(), 0..6),
            line_end in prop_oneof![Just("\n"), Just("\r\n"), Just("\r")],
            cuts in proptest::collection::vec(any::<prop::sample::Index>(), 0..8),
        ) {
            let body = encode(&events, line_end);
            let mut points: Vec<usize> = cuts.iter().map(|cut| cut.index(body.len() + 1)).collect();
            points.sort_unstable();
            let mut chunks = Vec::new();
            let mut start = 0;
            for point in points {
                chunks.push(&body[start..point]);
                start = point;
            }
            chunks.push(&body[start..]);

            prop_assert_eq!(parse_all(&chunks), events);
        }

        #[test]
        fn arbitrary_bytes_never_panic(chunks in proptest::collection::vec(proptest::collection::vec(any::<u8>(), 0..64), 0..8)) {
            let chunks: Vec<&[u8]> = chunks.iter().map(Vec::as_slice).collect();
            let _ = parse_all(&chunks);
        }
    }

    #[tokio::test]
    async fn events_reads_a_chunked_body_and_ends_on_a_transport_error() {
        let body: BodyStream = stream::iter([
            Ok(b"data: a\n".to_vec()),
            Ok(b"\ndata: b\n\n".to_vec()),
            Err(HttpError::Timeout),
            Ok(b"data: never\n\n".to_vec()),
        ])
        .boxed();

        let read: Vec<Result<SseEvent, HttpError>> = events(body).collect().await;

        assert_eq!(
            read,
            [
                Ok(event(None, "a")),
                Ok(event(None, "b")),
                Err(HttpError::Timeout)
            ]
        );
    }
}
