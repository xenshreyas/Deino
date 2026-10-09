//! How to start a worker process: which program, with which environment.

use std::ffi::OsString;
use std::io;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use crate::worker::{CPU_ONLY_FLAG, LOG_LEVEL_ENV, WORKER_FLAG};

/// ggml-metal's residency sets assert at process teardown; Handy turns them
/// off before Metal initializes (`lib.rs:626-638`). Set on every worker.
const METAL_NO_RESIDENCY: (&str, &str) = ("GGML_METAL_NO_RESIDENCY", "1");

/// Starts worker processes: the app's own executable in production, the fake
/// worker binary in tests.
#[derive(Clone, Debug)]
pub struct WorkerLauncher {
    program: PathBuf,
    env: Vec<(OsString, OsString)>,
}

impl WorkerLauncher {
    /// Workers run `program` with [`WORKER_FLAG`] as their first argument.
    pub fn new(program: impl Into<PathBuf>) -> Self {
        Self {
            program: program.into(),
            env: Vec::new(),
        }
    }

    /// Workers re-launch this very executable, so the two always agree on
    /// the protocol and the native libraries.
    pub fn current_exe() -> io::Result<Self> {
        Ok(Self::new(std::env::current_exe()?))
    }

    /// Adds an environment variable for every worker (tests inject faults).
    #[must_use]
    pub fn env(mut self, key: impl Into<OsString>, value: impl Into<OsString>) -> Self {
        self.env.push((key.into(), value.into()));
        self
    }

    /// The command for one worker, with piped stdio.
    pub(super) fn command(&self, cpu_only: bool) -> Command {
        let mut command = Command::new(&self.program);
        command.arg(WORKER_FLAG);
        if cpu_only {
            command.arg(CPU_ONLY_FLAG);
        }
        command
            .env(METAL_NO_RESIDENCY.0, METAL_NO_RESIDENCY.1)
            .env(LOG_LEVEL_ENV, log::max_level().as_str())
            .envs(self.env.iter().map(|(key, value)| (key, value)))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        command
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cpu_only_worker_gets_both_flags_and_metal_residency_off() {
        let command = WorkerLauncher::new("deino").env("EXTRA", "1").command(true);

        let args: Vec<_> = command.get_args().collect();
        assert_eq!(args, [WORKER_FLAG, CPU_ONLY_FLAG]);
        let env: Vec<_> = command.get_envs().collect();
        assert!(env.contains(&("GGML_METAL_NO_RESIDENCY".as_ref(), Some("1".as_ref()))));
        assert!(env.contains(&("EXTRA".as_ref(), Some("1".as_ref()))));
    }
}
