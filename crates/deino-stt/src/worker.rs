//! The worker process: the only place an [`Engine`] runs.
//!
//! The app re-launches its own executable with [`WORKER_FLAG`] and talks to it
//! over stdin and stdout with [`crate::protocol`] frames. A `GGML_ASSERT`
//! abort, a Metal driver fault or a hang then kills only the worker, never the
//! recording (PARITY_PLAN §4.0 "Process isolation"). Ported from Handy's
//! `engine_supervisor/worker.rs` (MIT).
//!
//! - stdout carries the protocol only: fd 1 is pointed at stderr before any
//!   native code runs, so a stray ggml print cannot corrupt a frame;
//! - a dedicated thread reads stdin, and the process ends the moment its
//!   parent closes stdin (unload, quit, or the app died), even mid-hang;
//! - logs go to stderr as tagged lines that the supervisor re-logs.

mod serve;
mod stdio;

use std::io::{self, BufReader};
use std::sync::mpsc;
use std::thread;

pub use serve::{Incoming, serve};
pub(crate) use stdio::LOG_LINE_PREFIX;

use crate::engine::Engine;
use crate::protocol::{Request, frame};

/// The hidden first argument that turns the executable into a worker.
pub const WORKER_FLAG: &str = "--transcribe-worker";

/// Worker argument: register the CPU backends only, so no GPU driver code
/// runs in a fallback worker at all.
pub const CPU_ONLY_FLAG: &str = "--cpu-only";

/// Environment variable carrying the app's log level to the worker.
pub const LOG_LEVEL_ENV: &str = "DEINO_STT_WORKER_LOG";

/// Exit code when the protocol pipes fail.
const EXIT_PIPE_FAILURE: i32 = 2;

/// Whether `args` (the process arguments, program name first) start a worker.
/// `main` checks this before anything else runs.
pub fn is_worker_invocation(mut args: impl Iterator<Item = std::ffi::OsString>) -> bool {
    args.nth(1).is_some_and(|arg| arg == WORKER_FLAG)
}

/// Runs the worker until its parent closes stdin, and returns the exit code.
pub fn run<E: Engine>(mut engine: E, cpu_only: bool) -> i32 {
    let mut output = match stdio::take_stdout_for_protocol() {
        Ok(output) => output,
        Err(error) => {
            stdio::report(&format!(
                "transcription worker: cannot claim stdout: {error}"
            ));
            return EXIT_PIPE_FAILURE;
        }
    };
    stdio::install_logger();
    stdio::raise_priority();
    let (requests_tx, requests) = mpsc::channel();
    let reader = thread::Builder::new()
        .name("stt-worker-in".into())
        .spawn(move || read_requests(&requests_tx));
    if let Err(error) = reader {
        log::error!("cannot start the request reader: {error}");
        return EXIT_PIPE_FAILURE;
    }
    // Registering a GPU backend runs driver code: this is where a broken
    // driver crashes or hangs, before the hello is answered.
    let init_error = engine.init(cpu_only).err();
    match serve(&mut engine, init_error.as_ref(), &requests, &mut output) {
        Ok(()) => 0,
        Err(error) => {
            log::error!("protocol write failed: {error}");
            EXIT_PIPE_FAILURE
        }
    }
}

/// Forwards requests to the main loop and ends the process when stdin
/// closes. Exiting without running C++ static destructors keeps a model that
/// is still alive from tripping ggml-metal's teardown asserts.
fn read_requests(requests: &mpsc::Sender<Incoming>) {
    let mut input = BufReader::new(io::stdin().lock());
    loop {
        match frame::read::<Request>(&mut input) {
            Ok(Some((request, pcm))) => {
                if requests.send(Incoming { request, pcm }).is_err() {
                    stdio::exit_now(1);
                }
            }
            Ok(None) => stdio::exit_now(0),
            Err(error) => {
                log::error!("protocol read failed: {error}");
                stdio::exit_now(1);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use super::*;

    fn args(list: &[&str]) -> impl Iterator<Item = OsString> {
        list.iter()
            .map(OsString::from)
            .collect::<Vec<_>>()
            .into_iter()
    }

    #[test]
    fn only_the_flag_in_first_position_starts_a_worker() {
        assert!(is_worker_invocation(args(&[
            "deino",
            WORKER_FLAG,
            CPU_ONLY_FLAG
        ])));
        assert!(!is_worker_invocation(args(&["deino"])));
        assert!(!is_worker_invocation(args(&[
            "deino",
            "--other",
            WORKER_FLAG
        ])));
    }
}
