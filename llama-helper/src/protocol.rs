//! The JSON-lines protocol between the app and the sidecar.
//!
//! The app writes one [`Request`] per line on stdin, tagged by `"type"`. The
//! sidecar answers on stdout with [`Response`] lines, one JSON object each:
//!
//! - `generate` → one `response` line (the legacy request, unchanged).
//! - `stream` and `grammar` → zero or more `token` lines, then one `done` or
//!   `error` line.
//! - `embed` → one `embeddings` or `error` line.
//! - `ping` → `pong`; `shutdown` → `goodbye`, then the process exits.
//!
//! The tests at the bottom pin the exact JSON, because the app codes against it.

use std::io::{self, Write};

use serde::{Deserialize, Serialize};

/// Tokens generated when a request does not say.
pub const DEFAULT_MAX_TOKENS: i32 = 512;

/// Context window, in tokens, when a request does not say.
pub const DEFAULT_CONTEXT_SIZE: u32 = 2048;

/// One request line from the app.
#[derive(Debug, PartialEq, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Request {
    /// Generate a whole completion and answer with one `response` line.
    Generate(CompletionRequest),
    /// Generate a completion and stream it as `token` lines.
    Stream(CompletionRequest),
    /// Like `stream`, with output constrained to a JSON schema.
    Grammar(GrammarRequest),
    /// Embed texts with an embedding model.
    Embed(EmbedRequest),
    /// Liveness check.
    Ping,
    /// Say goodbye and exit.
    Shutdown,
}

/// The fields shared by `generate`, `stream` and `grammar`.
#[derive(Debug, Default, PartialEq, Deserialize)]
pub struct CompletionRequest {
    /// The full prompt, chat template already applied by the app.
    pub prompt: String,
    /// Maximum tokens to generate ([`DEFAULT_MAX_TOKENS`] when absent).
    pub max_tokens: Option<i32>,
    /// Context window used when the model is loaded ([`DEFAULT_CONTEXT_SIZE`]).
    pub context_size: Option<u32>,
    /// The GGUF chat model; absent means "the model already loaded".
    pub model_path: Option<String>,
    /// Sampling parameters, sanitised by `SamplingConfig`.
    #[serde(flatten)]
    pub sampling: SamplingOptions,
    /// Texts that end generation; they are never part of the output.
    pub stop_tokens: Option<Vec<String>>,
}

/// Sampling parameters as they arrive; any of them may be absent or invalid.
#[derive(Debug, Default, Clone, Copy, PartialEq, Deserialize)]
pub struct SamplingOptions {
    /// Softmax temperature; zero or less means greedy decoding.
    pub temperature: Option<f32>,
    /// Keep only the `top_k` most likely tokens.
    pub top_k: Option<i32>,
    /// Nucleus sampling threshold in `(0, 1]`.
    pub top_p: Option<f32>,
    /// Flat penalty for any token already seen.
    pub presence_penalty: Option<f32>,
    /// Penalty that grows with how often a token was seen.
    pub frequency_penalty: Option<f32>,
    /// Multiplicative repetition penalty; 1 disables it.
    pub repeat_penalty: Option<f32>,
    /// How many recent tokens the penalties look at; 0 disables them.
    pub penalty_last_n: Option<i32>,
}

/// A `grammar` request: a completion constrained by a JSON schema.
#[derive(Debug, PartialEq, Deserialize)]
pub struct GrammarRequest {
    /// The same fields as `stream`.
    #[serde(flatten)]
    pub completion: CompletionRequest,
    /// Any JSON schema `llama.cpp` can turn into a GBNF grammar.
    pub json_schema: serde_json::Value,
}

/// An `embed` request.
#[derive(Debug, PartialEq, Deserialize)]
pub struct EmbedRequest {
    /// The GGUF embedding model, held in its own slot.
    pub model_path: String,
    /// Texts to embed; the answer has one vector per input, in order.
    pub inputs: Vec<String>,
    /// Context window, which is also the longest input in tokens.
    pub context_size: Option<u32>,
}

/// Why a streamed generation ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    /// The model produced an end-of-generation token.
    EndOfGeneration,
    /// `max_tokens` were generated.
    MaxTokens,
    /// One of the request's stop tokens appeared.
    StopToken,
}

/// One response line to the app.
#[derive(Debug, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Response {
    /// The answer to `generate`: the text, or an empty text and an error.
    #[serde(rename = "response")]
    Completion {
        /// The generated text, stop token removed.
        text: String,
        /// Why generation failed, if it did.
        error: Option<String>,
    },
    /// One decoded piece of a streamed completion.
    Token {
        /// Never a partial character and never any part of a stop token.
        text: String,
    },
    /// The end of a successful stream.
    Done {
        /// Tokens in the prompt.
        prompt_tokens: u32,
        /// Tokens generated, not counting the end-of-generation token.
        output_tokens: u32,
        /// Why generation ended.
        stop: StopReason,
    },
    /// The answer to `embed`.
    Embeddings {
        /// One L2-normalised vector per input, in order.
        vectors: Vec<Vec<f32>>,
        /// Tokens across all inputs.
        prompt_tokens: u32,
    },
    /// The answer to `ping`.
    Pong,
    /// Sent before the process exits.
    Goodbye,
    /// A failed request: unparsable, or a stream, grammar or embed failure.
    Error {
        /// What went wrong; never contains prompt text.
        message: String,
    },
}

/// Writes `response` as one JSON line and flushes, so the app sees it now.
pub fn write_response(out: &mut impl Write, response: &Response) -> io::Result<()> {
    serde_json::to_writer(&mut *out, response).map_err(io::Error::from)?;
    out.write_all(b"\n")?;
    out.flush()
}

#[cfg(test)]
#[path = "protocol_tests.rs"]
mod tests;
