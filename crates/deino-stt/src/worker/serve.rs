//! The worker's request loop: one [`Response`] for every [`Request`].
//!
//! Separate from the process plumbing so tests can drive it in-process with
//! a channel and a byte buffer. A stream borrows the engine, so while one is
//! open the loop hands over to [`serve_stream`] until it is finalized or reset.

use std::io::{self, Write};
use std::sync::mpsc::Receiver;

use crate::engine::{Engine, EngineStream};
use crate::protocol::{RemoteError, RemoteErrorKind, Request, Response, frame};

/// A request as read from the app, with its PCM.
#[derive(Debug)]
pub struct Incoming {
    /// The request.
    pub request: Request,
    /// The frame's PCM (empty for requests that carry none).
    pub pcm: Vec<f32>,
}

/// Serves requests until the channel closes. `init_error` is the backend
/// failure, if any, which the hello reports. Returns only on a write error or
/// when the requests end.
pub fn serve<E: Engine>(
    engine: &mut E,
    init_error: Option<&RemoteError>,
    requests: &Receiver<Incoming>,
    out: &mut impl Write,
) -> io::Result<()> {
    while let Ok(Incoming { request, pcm }) = requests.recv() {
        let response = match request {
            Request::Hello { warm_gpu } => hello(engine, init_error, warm_gpu),
            Request::Load(load) => engine
                .load(&load)
                .map_or_else(Response::Error, Response::Loaded),
            Request::Run(params) => engine
                .run(&pcm, &params)
                .map_or_else(Response::Error, Response::Transcript),
            Request::StreamBegin(params) => match engine.open_stream(&params) {
                Ok(stream) => {
                    frame::write(out, &Response::Ok, &[])?;
                    serve_stream(stream, requests, out)?;
                    continue;
                }
                Err(error) => Response::Error(error),
            },
            Request::Feed | Request::Finalize | Request::StreamReset => Response::Error(
                RemoteError::new(RemoteErrorKind::NoStream, "begin a stream first"),
            ),
        };
        frame::write(out, &response, &[])?;
    }
    Ok(())
}

fn hello<E: Engine>(engine: &mut E, init_error: Option<&RemoteError>, warm_gpu: bool) -> Response {
    if let Some(error) = init_error {
        return Response::Error(error.clone());
    }
    if warm_gpu {
        engine.warm_gpu();
    }
    Response::Hello
}

/// Serves one open stream until it is finalized or reset.
fn serve_stream(
    mut stream: impl EngineStream,
    requests: &Receiver<Incoming>,
    out: &mut impl Write,
) -> io::Result<()> {
    while let Ok(Incoming { request, pcm }) = requests.recv() {
        let (response, closed) = match request {
            Request::Feed => (
                stream
                    .feed(&pcm)
                    .map_or_else(Response::Error, Response::Fed),
                false,
            ),
            // Finalize ends the stream even when it fails.
            Request::Finalize => (
                stream
                    .finalize()
                    .map_or_else(Response::Error, Response::Finalized),
                true,
            ),
            Request::StreamReset => (Response::Ok, true),
            _ => (
                Response::Error(RemoteError::new(
                    RemoteErrorKind::StreamActive,
                    "finalize or reset the open stream first",
                )),
                false,
            ),
        };
        frame::write(out, &response, &[])?;
        if closed {
            break;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use super::*;
    use crate::engine::scripted::ScriptedEngine;
    use crate::protocol::{BackendChoice, BatchParams, LoadRequest, StreamParams};
    use crate::words::text_of;

    /// Serves `requests` against a fresh scripted engine and decodes the answers.
    fn exchange(requests: Vec<(Request, Vec<f32>)>) -> Vec<Response> {
        let model = tempfile::NamedTempFile::new().unwrap();
        let mut engine = ScriptedEngine::new(None);
        let (sender, receiver) = mpsc::channel();
        let load = LoadRequest {
            path: model.path().to_path_buf(),
            backend: BackendChoice::Auto,
        };
        for (request, pcm) in std::iter::once((Request::Load(load), Vec::new())).chain(requests) {
            sender.send(Incoming { request, pcm }).unwrap();
        }
        drop(sender);
        let mut out = Vec::new();
        serve(&mut engine, None, &receiver, &mut out).unwrap();
        let mut input = out.as_slice();
        let mut responses = Vec::new();
        while let Some((response, _)) = frame::read::<Response>(&mut input).unwrap() {
            responses.push(response);
        }
        responses.into_iter().skip(1).collect()
    }

    #[test]
    fn a_stream_answers_every_feed_and_closes_on_finalize() {
        let loud = vec![0.5_f32; 6400];
        let responses = exchange(vec![
            (Request::StreamBegin(StreamParams::default()), Vec::new()),
            (Request::Feed, loud.clone()),
            (Request::Feed, loud),
            (Request::Finalize, Vec::new()),
            (Request::Feed, Vec::new()),
        ]);

        assert_eq!(responses[0], Response::Ok);
        let Response::Fed(delta) = &responses[2] else {
            panic!("{responses:?}")
        };
        assert_eq!(text_of(&delta.tokens), "w0");
        assert!(matches!(responses[3], Response::Finalized(_)));
        let Response::Error(error) = &responses[4] else {
            panic!("{responses:?}")
        };
        assert_eq!(error.kind, RemoteErrorKind::NoStream);
    }

    #[test]
    fn a_run_while_a_stream_is_open_is_refused_without_closing_it() {
        let responses = exchange(vec![
            (Request::StreamBegin(StreamParams::default()), Vec::new()),
            (Request::Run(BatchParams::default()), vec![0.5; 6400]),
            (Request::StreamReset, Vec::new()),
            (Request::Run(BatchParams::default()), vec![0.5; 6400]),
        ]);

        let Response::Error(error) = &responses[1] else {
            panic!("{responses:?}")
        };
        assert_eq!(error.kind, RemoteErrorKind::StreamActive);
        assert_eq!(responses[2], Response::Ok);
        assert!(matches!(&responses[3], Response::Transcript(t) if t.text == "w0"));
    }

    #[test]
    fn a_failed_backend_init_is_reported_by_the_hello() {
        let mut engine = ScriptedEngine::new(None);
        let (sender, receiver) = mpsc::channel();
        sender
            .send(Incoming {
                request: Request::Hello { warm_gpu: true },
                pcm: Vec::new(),
            })
            .unwrap();
        drop(sender);
        let failure = RemoteError::new(RemoteErrorKind::BackendInit, "no device");
        let mut out = Vec::new();

        serve(&mut engine, Some(&failure), &receiver, &mut out).unwrap();

        let (response, _) = frame::read::<Response>(&mut out.as_slice())
            .unwrap()
            .unwrap();
        assert_eq!(response, Response::Error(failure));
    }
}
