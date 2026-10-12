//! The shell step against a real `/bin/sh`: off by default, the payload on
//! stdin in the meeting's folder, exit statuses, and the time limit that
//! kills the script's whole process group.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::DateTime;
use deino_domain::{AutomationId, AutomationRunId, ConditionGroups, FakeClock, MeetingId, Millis};
use deino_export::{ActionItemExport, MeetingExport, Utterance};
use deino_testkit::at;

use super::{MeetingFolders, ShellStep};
use crate::automations::{
    Action, Automation, FixedShellSwitch, MeetingSource, PayloadFields, Step, StepContext,
    StepError, StepOutput, Trigger,
};
use crate::export::ExportError;

const MEETING: &str = "meeting-shell";

/// One fixed meeting, as the export assembler would read it.
struct OneMeeting;

#[async_trait]
impl MeetingSource for OneMeeting {
    async fn export(&self, meeting: &MeetingId) -> Result<MeetingExport, ExportError> {
        Ok(MeetingExport {
            id: meeting.clone(),
            title: "Acme sync".into(),
            started_at: DateTime::parse_from_rfc3339("2026-10-12T09:00:00-07:00").unwrap(),
            duration: None,
            attendees: Vec::new(),
            tags: vec!["Sales".into()],
            folders: Vec::new(),
            notes: Some("## Summary".into()),
            my_notes: None,
            action_items: vec![ActionItemExport {
                title: "Send the quote".into(),
                description: String::new(),
                assignee: None,
                due: None,
                done: false,
            }],
            insights: Vec::new(),
            transcript: vec![Utterance {
                speaker: "Me".into(),
                is_me: true,
                start: Millis(0),
                end: Millis(900),
                text: "Hello".into(),
            }],
        })
    }

    async fn audio_path(&self, _meeting: &MeetingId) -> Result<Option<String>, ExportError> {
        Ok(Some("/recordings/meeting-shell/audio.m4a".into()))
    }
}

/// Every meeting's folder under one temporary directory.
struct TempFolders(PathBuf);

impl MeetingFolders for TempFolders {
    fn folder(&self, meeting: &MeetingId) -> PathBuf {
        self.0.join(meeting.as_str())
    }
}

struct Setup {
    _dir: tempfile::TempDir,
    folder: PathBuf,
    automation: Automation,
}

fn setup() -> Setup {
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path().join(MEETING);
    let clock = FakeClock::new(at("2026-10-12T09:00:00Z"));
    let automation = Automation {
        id: AutomationId::generate(&clock),
        name: "Hook".into(),
        enabled: true,
        auto_run: true,
        stop_on_failure: false,
        conditions: ConditionGroups::every_meeting(),
        steps: Vec::new(),
        created_at: at("2026-10-12T09:00:00Z"),
        updated_at: at("2026-10-12T09:00:00Z"),
    };
    Setup {
        folder,
        automation,
        _dir: dir,
    }
}

fn step(setup: &Setup, allowed: bool) -> ShellStep {
    let root = setup.folder.parent().unwrap().to_path_buf();
    ShellStep::over(
        Arc::new(FixedShellSwitch(allowed)),
        Arc::new(OneMeeting),
        Arc::new(TempFolders(root)),
    )
}

async fn run(step: &ShellStep, setup: &Setup, command: &str) -> Result<StepOutput, StepError> {
    let action = Action::RunShell {
        command: command.into(),
        fields: PayloadFields::default(),
    };
    let run_id = AutomationRunId::generate(&FakeClock::new(at("2026-10-12T09:00:00Z")));
    let meeting = MeetingId::new(MEETING);
    let earlier = [StepOutput::Notified];
    let ctx = StepContext {
        automation: &setup.automation,
        run_id: &run_id,
        meeting: &meeting,
        trigger: Trigger::Pipeline,
        earlier: &earlier,
        final_attempt: false,
    };
    step.run(&action, ctx).await
}

#[tokio::test]
async fn scripts_do_not_run_until_the_user_allows_them() {
    let setup = setup();

    let result = run(&step(&setup, false), &setup, "touch ran").await;

    assert!(matches!(result, Err(StepError::ShellDisabled)));
    assert!(!setup.folder.join("ran").exists());
}

#[tokio::test]
async fn the_script_reads_the_payload_on_stdin_in_the_meeting_folder() {
    let setup = setup();

    let output = run(
        &step(&setup, true),
        &setup,
        "cat > payload.json; pwd; echo done",
    )
    .await;

    assert!(matches!(output, Ok(StepOutput::Script { ref output }) if output.ends_with("done")));
    let json: serde_json::Value =
        serde_json::from_slice(&std::fs::read(setup.folder.join("payload.json")).unwrap()).unwrap();
    assert_eq!(json["format"], "deino.automation");
    assert_eq!(json["automation"]["name"], "Hook");
    assert_eq!(json["trigger"]["kind"], "pipeline");
    assert_eq!(json["meeting"]["title"], "Acme sync");
    assert_eq!(json["notes"], "## Summary");
    assert_eq!(json["actionItems"][0]["title"], "Send the quote");
    assert_eq!(json["link"], "deino://meetings/meeting-shell");
    assert!(
        json.get("transcript").is_none(),
        "unselected fields are left out"
    );
    assert!(json.get("audioPath").is_none());
    assert_eq!(json["steps"][0]["kind"], "notified");
}

#[tokio::test]
async fn a_failing_script_reports_its_status_and_the_end_of_its_stderr() {
    let setup = setup();

    let result = run(&step(&setup, true), &setup, "echo boom >&2; exit 3").await;

    let Err(StepError::ExitStatus { code, stderr }) = result else {
        panic!("{result:?}")
    };
    assert_eq!((code, stderr.as_str()), (Some(3), "boom"));
}

#[cfg(unix)]
#[tokio::test]
async fn a_script_past_its_time_limit_is_stopped_with_its_children() {
    let setup = setup();
    let step = step(&setup, true).with_timeout(Duration::from_millis(300));

    let result = run(&step, &setup, "sleep 30 & echo $! > child.pid; wait").await;

    assert!(
        matches!(result, Err(StepError::Timeout { .. })),
        "{result:?}"
    );
    let pid: i32 = std::fs::read_to_string(setup.folder.join("child.pid"))
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    let child = rustix::process::Pid::from_raw(pid).unwrap();
    for _ in 0..50 {
        if rustix::process::test_kill_process(child).is_err() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("the script's child {pid} outlived the time limit");
}

#[test]
fn the_default_time_limit_is_a_minute() {
    assert_eq!(super::SHELL_TIMEOUT, Duration::from_mins(1));
}
