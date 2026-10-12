//! [`ShellStep`]: runs the user's command after a meeting, the way Muesli's
//! post-meeting hooks do.
//!
//! - **Off by default:** Settings → Automations → "Allow shell scripts"
//!   ([`ShellSwitch`]) must be on, checked at every run.
//! - The command runs with `/bin/sh -c` in the meeting's folder
//!   ([`MeetingFolders`]), with the [`AutomationPayload`](crate::automations::AutomationPayload)
//!   as JSON on stdin.
//! - It runs in its own process group. After [`SHELL_TIMEOUT`] the whole
//!   group is killed, so a script's children cannot outlive it.
//! - A zero exit is success; the end of stdout is kept as the step's
//!   output, the end of stderr in a failure's message. Neither is logged.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use deino_domain::MeetingId;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use tokio::process::{Child, Command};

use super::LocalStepPorts;
use crate::automations::{
    Action, ActionKind, MeetingSource, Step, StepContext, StepError, StepOutput, build_payload,
};
use crate::settings::SettingsService;

/// How long a script may run before its process group is killed.
pub const SHELL_TIMEOUT: Duration = Duration::from_mins(1);
/// How much of a script's output the run log keeps, in characters.
const OUTPUT_TAIL_CHARS: usize = 400;
/// The shell commands run with.
const SHELL: &str = "/bin/sh";

/// Whether the user allows shell scripts.
#[async_trait]
pub trait ShellSwitch: Send + Sync {
    /// True when Settings → Automations allows shell scripts.
    async fn shell_allowed(&self) -> bool;
}

#[async_trait]
impl ShellSwitch for SettingsService {
    async fn shell_allowed(&self) -> bool {
        self.get().await.automations.shell_scripts
    }
}

/// Where each meeting's folder is.
pub trait MeetingFolders: Send + Sync {
    /// The meeting's folder; created before a script runs in it.
    fn folder(&self, meeting: &MeetingId) -> PathBuf;
}

/// Each meeting's recording folder: `<recordings>/<meeting id>/`.
pub struct RecordingFolders(pub PathBuf);

impl MeetingFolders for RecordingFolders {
    fn folder(&self, meeting: &MeetingId) -> PathBuf {
        self.0.join(meeting.as_str())
    }
}

/// Runs [`Action::RunShell`].
pub struct ShellStep {
    switch: Arc<dyn ShellSwitch>,
    meetings: Arc<dyn MeetingSource>,
    folders: Arc<dyn MeetingFolders>,
    timeout: Duration,
}

impl ShellStep {
    /// The step over the switch, the meetings and their folders.
    pub fn new(ports: &LocalStepPorts) -> Self {
        Self::over(
            Arc::clone(&ports.shell_switch),
            Arc::clone(&ports.meetings),
            Arc::clone(&ports.folders),
        )
    }

    /// The step over exactly these ports, with the [`SHELL_TIMEOUT`].
    pub fn over(
        switch: Arc<dyn ShellSwitch>,
        meetings: Arc<dyn MeetingSource>,
        folders: Arc<dyn MeetingFolders>,
    ) -> Self {
        Self {
            switch,
            meetings,
            folders,
            timeout: SHELL_TIMEOUT,
        }
    }

    /// The same step with a different time limit (tests).
    #[must_use]
    pub fn with_timeout(self, timeout: Duration) -> Self {
        Self { timeout, ..self }
    }
}

#[async_trait]
impl Step for ShellStep {
    fn kind(&self) -> ActionKind {
        ActionKind::RunShell
    }

    async fn run(&self, action: &Action, ctx: StepContext<'_>) -> Result<StepOutput, StepError> {
        let Action::RunShell { command, fields } = action else {
            return Err(StepError::Unavailable);
        };
        if !self.switch.shell_allowed().await {
            return Err(StepError::ShellDisabled);
        }
        let payload = build_payload(self.meetings.as_ref(), ctx, *fields).await?;
        let stdin = serde_json::to_vec(&payload)
            .map_err(|error| StepError::Io("encode the payload", error.into()))?;
        let folder = self.folders.folder(ctx.meeting);
        tokio::fs::create_dir_all(&folder)
            .await
            .map_err(|error| StepError::Io("create the meeting folder", error))?;
        let script = Script {
            command,
            folder: &folder,
            stdin,
            timeout: self.timeout,
        };
        let output = script.run().await?;
        Ok(StepOutput::Script { output })
    }
}

/// One run of a command.
struct Script<'a> {
    command: &'a str,
    folder: &'a Path,
    stdin: Vec<u8>,
    timeout: Duration,
}

impl Script<'_> {
    /// Runs the command; the tail of its stdout on success.
    async fn run(self) -> Result<String, StepError> {
        let mut child = self
            .spawn()
            .map_err(|error| StepError::Io("start the script", error))?;
        let group = child.id();
        let finished =
            tokio::time::timeout(self.timeout, communicate(&mut child, self.stdin)).await;
        let Ok(finished) = finished else {
            kill_group(group);
            if let Err(error) = child.kill().await {
                log::warn!(target: "automations", "could not reap a timed-out script: {error}");
            }
            return Err(StepError::Timeout {
                seconds: self.timeout.as_secs(),
            });
        };
        let (status, stdout, stderr) =
            finished.map_err(|error| StepError::Io("run the script", error))?;
        if status.success() {
            Ok(stdout)
        } else {
            Err(StepError::ExitStatus {
                code: status.code(),
                stderr,
            })
        }
    }

    /// The child, in a process group of its own, with every pipe open.
    fn spawn(&self) -> std::io::Result<Child> {
        let mut command = Command::new(SHELL);
        command
            .arg("-c")
            .arg(self.command)
            .current_dir(self.folder)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        #[cfg(unix)]
        command.process_group(0);
        command.spawn()
    }
}

/// Writes `stdin`, reads both outputs and waits for the exit, all at once
/// so a script that prints before it reads cannot deadlock on a full pipe.
async fn communicate(
    child: &mut Child,
    stdin: Vec<u8>,
) -> std::io::Result<(std::process::ExitStatus, String, String)> {
    let input = child.stdin.take();
    let write = async move {
        if let Some(mut input) = input {
            // A script that never reads its stdin closes the pipe; that is its choice.
            if input.write_all(&stdin).await.is_err() {
                log::debug!(target: "automations", "the script did not read its payload");
            }
        }
    };
    let ((), stdout, stderr) = tokio::join!(
        write,
        read_tail(child.stdout.take()),
        read_tail(child.stderr.take())
    );
    let status = child.wait().await?;
    Ok((status, stdout, stderr))
}

/// The last [`OUTPUT_TAIL_CHARS`] characters `pipe` printed, trimmed.
async fn read_tail(pipe: Option<impl AsyncRead + Unpin>) -> String {
    let Some(mut pipe) = pipe else {
        return String::new();
    };
    let mut bytes = Vec::new();
    if pipe.read_to_end(&mut bytes).await.is_err() {
        return String::new();
    }
    let text = String::from_utf8_lossy(&bytes);
    let text = text.trim();
    let skip = text.chars().count().saturating_sub(OUTPUT_TAIL_CHARS);
    text.chars().skip(skip).collect()
}

/// Kills the script's whole process group (its children included).
fn kill_group(group: Option<u32>) {
    #[cfg(unix)]
    {
        use rustix::process::{Pid, Signal, kill_process_group};
        let pid = group
            .and_then(|id| i32::try_from(id).ok())
            .and_then(Pid::from_raw);
        if let Some(pid) = pid
            && let Err(error) = kill_process_group(pid, Signal::KILL)
        {
            log::warn!(target: "automations", "could not stop a timed-out script's group: {error}");
        }
    }
    #[cfg(not(unix))]
    let _unused = group;
}
