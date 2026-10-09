//! The request loop: read a line, dispatch it, write the response lines.
//!
//! [`Server`] reads requests from any `BufRead` and writes responses to any
//! `Write`, so tests drive it with in-memory buffers. Before each read it
//! checks the [`IdleTimer`] and says goodbye if the sidecar sat idle too long.
//! It exits on `shutdown`, at the end of input, or when stdin fails.

use std::io::{self, BufRead, Write};
use std::path::PathBuf;

use crate::clock::IdleTimer;
use crate::completion::{self, CompletionJob};
use crate::diag::diag;
use crate::embed;
use crate::generate::{CollectSink, TokenSink};
use crate::model::{ModelStore, SlotKind};
use crate::protocol::{
    CompletionRequest, DEFAULT_CONTEXT_SIZE, EmbedRequest, Request, Response, write_response,
};

/// Whether to keep reading requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Flow {
    Continue,
    Exit,
}

/// Answers requests until told to stop.
#[derive(Debug)]
pub struct Server<W: Write> {
    out: W,
    store: ModelStore,
    idle: IdleTimer,
}

impl<W: Write> Server<W> {
    /// A server writing to `out` that exits after `idle_timeout_secs` idle.
    pub fn new(out: W, idle_timeout_secs: u64) -> Self {
        Self {
            out,
            store: ModelStore::default(),
            idle: IdleTimer::new(idle_timeout_secs),
        }
    }

    /// Serves requests from `input` until shutdown, idle timeout or end of input.
    /// Fails only when a response cannot be written.
    pub fn run(&mut self, mut input: impl BufRead) -> io::Result<()> {
        let mut line = String::new();
        loop {
            if self.idle.expired() {
                diag!("Idle timeout reached, shutting down");
                return self.send(&Response::Goodbye);
            }
            line.clear();
            match input.read_line(&mut line) {
                Ok(0) => {
                    diag!("EOF received, shutting down");
                    return Ok(());
                }
                Ok(_) => {}
                Err(error) => {
                    diag!("Error reading stdin: {error}");
                    return Ok(());
                }
            }
            if self.handle_line(line.trim())? == Flow::Exit {
                return Ok(());
            }
        }
    }

    /// Parses and answers one line; blank lines are skipped.
    fn handle_line(&mut self, line: &str) -> io::Result<Flow> {
        if line.is_empty() {
            return Ok(Flow::Continue);
        }
        match serde_json::from_str::<Request>(line) {
            Ok(request) => self.dispatch(request),
            Err(error) => {
                // The message can quote request values, so stderr gets only where it failed.
                diag!(
                    "Failed to parse request ({:?} error at column {})",
                    error.classify(),
                    error.column()
                );
                let message = format!("Invalid request: {error}");
                self.send(&Response::Error { message })?;
                Ok(Flow::Continue)
            }
        }
    }

    /// Answers one parsed request.
    fn dispatch(&mut self, request: Request) -> io::Result<Flow> {
        match request {
            Request::Generate(request) => self.generate(request)?,
            Request::Stream(request) => self.stream(CompletionJob {
                request,
                schema: None,
            })?,
            Request::Grammar(request) => self.stream(CompletionJob {
                request: request.completion,
                schema: Some(request.json_schema),
            })?,
            Request::Embed(request) => self.embed(request)?,
            Request::Ping => self.send(&Response::Pong)?,
            Request::Shutdown => {
                diag!("Shutdown requested");
                self.send(&Response::Goodbye)?;
                return Ok(Flow::Exit);
            }
        }
        self.idle.touch();
        Ok(Flow::Continue)
    }

    /// `generate`: one `response` line with the whole text or an error.
    fn generate(&mut self, request: CompletionRequest) -> io::Result<()> {
        let mut sink = CollectSink::default();
        let job = CompletionJob {
            request,
            schema: None,
        };
        let response = match completion::run(&mut self.store, job, &mut sink) {
            Ok(_) => Response::Completion {
                text: sink.text,
                error: None,
            },
            Err(error) => Response::Completion {
                text: String::new(),
                error: Some(error.legacy_message()),
            },
        };
        self.send(&response)
    }

    /// `stream` and `grammar`: `token` lines, then `done` or `error`.
    fn stream(&mut self, job: CompletionJob) -> io::Result<()> {
        let mut sink = LineSink { out: &mut self.out };
        let response = match completion::run(&mut self.store, job, &mut sink) {
            Ok(outcome) => Response::Done {
                prompt_tokens: outcome.prompt_tokens,
                output_tokens: outcome.finish.output_tokens,
                stop: outcome.finish.stop,
            },
            Err(error) => Response::Error {
                message: error.message(),
            },
        };
        self.send(&response)
    }

    /// `embed`: one `embeddings` or `error` line.
    fn embed(&mut self, request: EmbedRequest) -> io::Result<()> {
        let EmbedRequest {
            model_path,
            inputs,
            context_size,
        } = request;
        let context_size = context_size.unwrap_or(DEFAULT_CONTEXT_SIZE);
        let path = PathBuf::from(model_path);
        let response = match self.store.load(SlotKind::Embedding, path, context_size) {
            Err(error) => Response::Error {
                message: format!("Failed to load model: {error:#}"),
            },
            Ok(()) => match self.embed_inputs(&inputs) {
                Ok(embeddings) => Response::Embeddings {
                    vectors: embeddings.vectors,
                    prompt_tokens: embeddings.prompt_tokens,
                },
                Err(error) => Response::Error {
                    message: format!("Embedding failed: {error:#}"),
                },
            },
        };
        self.send(&response)
    }

    /// Embeds `inputs` with the loaded embedding model.
    fn embed_inputs(&self, inputs: &[String]) -> anyhow::Result<embed::Embeddings> {
        embed::embed(self.store.engine(SlotKind::Embedding)?, inputs)
    }

    /// Writes one response line.
    fn send(&mut self, response: &Response) -> io::Result<()> {
        write_response(&mut self.out, response)
    }
}

/// Writes each piece of a streamed completion as a `token` line.
struct LineSink<'w, W: Write> {
    out: &'w mut W,
}

impl<W: Write> TokenSink for LineSink<'_, W> {
    fn emit(&mut self, text: &str) -> anyhow::Result<()> {
        let token = Response::Token {
            text: text.to_owned(),
        };
        Ok(write_response(self.out, &token)?)
    }
}

#[cfg(test)]
#[path = "server_tests.rs"]
mod tests;
