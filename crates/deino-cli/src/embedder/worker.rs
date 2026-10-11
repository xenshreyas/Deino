//! The thread that owns one llama-helper process and answers embed requests
//! in order. It always reads a request's reply, so a caller that stopped
//! waiting (a search past its deadline) cannot leave a stale line for the
//! next request; a helper that died (its idle timeout) is started again.

use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex, PoisonError};

use tokio::sync::oneshot;

/// One request line and where its reply goes.
pub(super) struct Job {
    /// The JSON request, without its newline.
    pub(super) line: String,
    /// The reply line, or why there is none.
    pub(super) reply: oneshot::Sender<Result<String, String>>,
}

/// The running helper; shared so the embedder can kill it on drop.
pub(super) type Slot = Arc<Mutex<Option<Child>>>;

/// A started helper's pipes.
struct Pipes {
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

/// Serves `jobs` until the embedder hangs up, then stops the helper.
pub(super) fn serve(helper: &Path, jobs: &Receiver<Job>, slot: &Slot) {
    let mut pipes: Option<Pipes> = None;
    while let Ok(job) = jobs.recv() {
        let reply = exchange(helper, &mut pipes, slot, &job.line)
            .or_else(|_| {
                // The helper exits after an idle timeout: start it once more.
                pipes = None;
                exchange(helper, &mut pipes, slot, &job.line)
            })
            .map_err(|error| error.to_string());
        // The caller may have stopped waiting; the reply was still read.
        let _unwanted = job.reply.send(reply);
    }
    stop(slot);
}

/// Writes `line` and reads its reply, starting the helper if needed.
fn exchange(
    helper: &Path,
    pipes: &mut Option<Pipes>,
    slot: &Slot,
    line: &str,
) -> std::io::Result<String> {
    if pipes.is_none() {
        *pipes = Some(start(helper, slot)?);
    }
    let Some(started) = pipes.as_mut() else {
        return Err(std::io::Error::other("the helper did not start"));
    };
    writeln!(started.stdin, "{line}")?;
    started.stdin.flush()?;
    let mut reply = String::new();
    if started.stdout.read_line(&mut reply)? == 0 {
        *pipes = None;
        return Err(std::io::Error::other("the helper stopped"));
    }
    Ok(reply)
}

/// Starts the helper, replacing (and stopping) any earlier one.
fn start(helper: &Path, slot: &Slot) -> std::io::Result<Pipes> {
    stop(slot);
    let mut child = Command::new(helper)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    let stdin = child.stdin.take();
    let stdout = child.stdout.take();
    *lock(slot) = Some(child);
    match (stdin, stdout) {
        (Some(stdin), Some(stdout)) => Ok(Pipes {
            stdin,
            stdout: BufReader::new(stdout),
        }),
        _ => Err(std::io::Error::other("the helper has no pipes")),
    }
}

/// Kills and reaps the helper, if one runs.
pub(super) fn stop(slot: &Slot) {
    if let Some(mut child) = lock(slot).take() {
        let _killed = child.kill();
        let _reaped = child.wait();
    }
}

fn lock(slot: &Slot) -> std::sync::MutexGuard<'_, Option<Child>> {
    slot.lock().unwrap_or_else(PoisonError::into_inner)
}
