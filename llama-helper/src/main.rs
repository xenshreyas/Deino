//! `llama-helper`: Deino's built-in local LLM.
//!
//! A sidecar process that runs llama.cpp and speaks JSON lines: one request
//! per line on stdin, response lines on stdout (see [`protocol`]). stdout
//! carries only protocol lines; diagnostics go to stderr through [`diag`] and
//! never contain prompt text, generated text or embeddings.
//!
//! - [`server`] reads requests and dispatches them.
//! - [`completion`] runs `generate`, `stream` and `grammar`: [`model`] loads
//!   the chat model, [`grammar`] and [`sampling`] build the sampler,
//!   [`source`] drives llama.cpp and [`generate`] with [`stop`] decide what
//!   text to emit.
//! - [`embed`] answers `embed` with a separate embedding model slot.
//! - [`gpu`] plans GPU offload; [`clock`] is the only reader of system time.

mod clock;
mod completion;
mod diag;
mod embed;
mod generate;
mod gpu;
mod grammar;
mod model;
mod protocol;
mod sampling;
mod server;
mod source;
mod stop;

use std::io;

use crate::diag::diag;

/// The environment variable that sets the idle timeout, in seconds.
const IDLE_TIMEOUT_ENV: &str = "LLAMA_IDLE_TIMEOUT";
/// The idle timeout when the environment does not set one: five minutes.
const DEFAULT_IDLE_TIMEOUT_SECS: u64 = 300;

fn main() -> anyhow::Result<()> {
    let idle_timeout_secs = std::env::var(IDLE_TIMEOUT_ENV)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(DEFAULT_IDLE_TIMEOUT_SECS);
    diag!("llama-helper starting (idle timeout: {idle_timeout_secs}s)");
    let mut server = server::Server::new(io::stdout().lock(), idle_timeout_secs);
    server.run(io::stdin().lock())?;
    diag!("llama-helper exiting");
    Ok(())
}
