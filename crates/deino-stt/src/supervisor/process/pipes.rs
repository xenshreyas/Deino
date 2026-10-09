//! The three threads that move a worker's stdin, stdout and stderr.

use std::collections::VecDeque;
use std::io::{self, BufRead, BufReader, Write};
use std::process::{Child, ChildStderr, ChildStdin, ChildStdout};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

use super::super::lock;
use crate::protocol::{Response, frame};
use crate::worker::LOG_LINE_PREFIX;

/// Native stderr lines kept for crash reports.
const NATIVE_TAIL_LINES: usize = 64;

/// Log target for re-logged worker lines.
const WORKER_TARGET: &str = "deino_stt::worker";

/// The parent's ends of a worker's pipes.
pub(super) struct Pipes {
    /// Frames queued for the worker's stdin.
    pub stdin: Sender<Vec<u8>>,
    /// Responses read from the worker's stdout; disconnects when it closes.
    pub responses: Receiver<Response>,
    /// Signalled when the worker's stderr closes.
    pub stderr_done: Receiver<()>,
    /// The last native (non-log) stderr lines.
    pub native_tail: Arc<Mutex<VecDeque<String>>>,
}

/// Takes the child's stdio and starts the threads that serve it.
pub(super) fn take(child: &mut Child) -> io::Result<Pipes> {
    let missing = || io::Error::other("worker stdio is not piped");
    let stdin = child.stdin.take().ok_or_else(missing)?;
    let stdout = child.stdout.take().ok_or_else(missing)?;
    let stderr = child.stderr.take().ok_or_else(missing)?;
    let (frames_tx, frames) = mpsc::channel();
    let (responses_tx, responses) = mpsc::channel();
    let (stderr_done_tx, stderr_done) = mpsc::channel();
    let native_tail = Arc::new(Mutex::new(VecDeque::with_capacity(NATIVE_TAIL_LINES)));
    let tail = Arc::clone(&native_tail);
    spawn("stt-worker-stdin", move || write_frames(stdin, &frames))?;
    spawn("stt-worker-stdout", move || {
        read_responses(stdout, &responses_tx);
    })?;
    spawn("stt-worker-stderr", move || {
        drain_stderr(stderr, &tail);
        let _closed = stderr_done_tx.send(());
    })?;
    Ok(Pipes {
        stdin: frames_tx,
        responses,
        stderr_done,
        native_tail,
    })
}

fn spawn(name: &str, body: impl FnOnce() + Send + 'static) -> io::Result<()> {
    thread::Builder::new()
        .name(name.into())
        .spawn(body)
        .map(drop)
}

/// Ends when the stdin queue closes (dropping stdin sends EOF) or the worker
/// is gone.
fn write_frames(mut stdin: ChildStdin, frames: &Receiver<Vec<u8>>) {
    for frame in frames {
        if stdin
            .write_all(&frame)
            .and_then(|()| stdin.flush())
            .is_err()
        {
            break;
        }
    }
}

fn read_responses(stdout: ChildStdout, responses: &Sender<Response>) {
    let mut stdout = BufReader::new(stdout);
    loop {
        match frame::read::<Response>(&mut stdout) {
            Ok(Some((response, _))) => {
                if responses.send(response).is_err() {
                    break;
                }
            }
            Ok(None) => break,
            Err(error) => {
                log::error!("transcription worker protocol error: {error}");
                break;
            }
        }
    }
}

/// Splits on raw bytes: native code may write non-UTF-8, and `lines()` would
/// stop there and lose every later line, the abort message included.
fn drain_stderr(stderr: ChildStderr, tail: &Mutex<VecDeque<String>>) {
    for line in BufReader::new(stderr).split(b'\n') {
        let Ok(line) = line else { break };
        let line = String::from_utf8_lossy(&line);
        let line = line.strip_suffix('\r').unwrap_or(&line);
        if forward_log_line(line) {
            continue;
        }
        log::debug!(target: WORKER_TARGET, "{line}");
        let mut tail = lock(tail);
        if tail.len() == NATIVE_TAIL_LINES {
            tail.pop_front();
        }
        tail.push_back(line.to_string());
    }
}

/// Re-logs a worker log line at its own level. False for raw native output.
fn forward_log_line(line: &str) -> bool {
    let Some(rest) = line.strip_prefix(LOG_LINE_PREFIX) else {
        return false;
    };
    let mut parts = rest.splitn(3, '\t');
    let (Some(level), Some(target), Some(message)) = (parts.next(), parts.next(), parts.next())
    else {
        return false;
    };
    let level = level.parse().unwrap_or(log::Level::Info);
    log::log!(target: WORKER_TARGET, level, "[{target}] {message}");
    true
}
