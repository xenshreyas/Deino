//! Health rules with a fake clock: silent tracks, speech without text, a
//! crashed engine, low and full disks.

use super::*;

const SECOND: Duration = Duration::from_secs(1);

/// A healthy recording's vitals, ticking second by second.
struct Rig {
    monitor: HealthMonitor,
    vitals: Vitals,
    now: HostTime,
}

impl Rig {
    fn new() -> Self {
        let now = HostTime(1_000_000_000);
        let track = TrackVitals {
            frames: Frames::ZERO,
            status: TrackStatus::Live,
        };
        let engine = Some(LaneStatus {
            recoveries: 0,
            streaming: true,
        });
        Self {
            monitor: HealthMonitor::new(now),
            vitals: Vitals {
                tracks: PerLane {
                    me: track,
                    them: track,
                },
                unanswered: Millis::ZERO,
                engines: PerLane {
                    me: engine,
                    them: engine,
                },
                free_disk: Some(Bytes::gib(100)),
            },
            now,
        }
    }

    /// One second passes; `flowing` tracks deliver a second of audio.
    fn tick(&mut self, flowing: &[Lane]) -> Vec<HealthIssue> {
        self.now = self.now.plus(SECOND);
        for lane in flowing {
            self.vitals.tracks.get_mut(*lane).frames += Frames(48_000);
        }
        self.monitor.evaluate(&self.vitals, self.now);
        self.monitor.issues().to_vec()
    }

    fn ticks(&mut self, seconds: usize, flowing: &[Lane]) -> Vec<HealthIssue> {
        (0..seconds).fold(Vec::new(), |_, _| self.tick(flowing))
    }
}

fn no_audio(channel: Channel) -> HealthIssue {
    issue(IssueKind::NoAudio, Some(channel))
}

#[test]
fn a_healthy_recording_has_no_issues() {
    let mut rig = Rig::new();
    assert!(rig.ticks(60, &Lane::ALL).is_empty());
}

#[test]
fn a_track_without_frames_for_five_seconds_is_silent_until_it_flows_again() {
    let mut rig = Rig::new();
    assert!(rig.ticks(4, &[Lane::Me]).is_empty());
    assert_eq!(rig.tick(&[Lane::Me]), [no_audio(Channel::System)]);

    assert!(rig.tick(&Lane::ALL).is_empty());
}

#[test]
fn an_unplugged_microphone_is_reported_within_ten_seconds() {
    let mut rig = Rig::new();
    rig.ticks(10, &Lane::ALL);
    rig.vitals.tracks.me.status = TrackStatus::Recovering;

    let issues = rig.ticks(5, &[Lane::Them]);

    assert_eq!(issues, [no_audio(Channel::Mic)]);
}

#[test]
fn a_track_switched_off_or_paused_is_never_silent() {
    let mut rig = Rig::new();
    rig.vitals.tracks.them.status = TrackStatus::Off;
    assert!(rig.ticks(30, &[Lane::Me]).is_empty());

    rig.vitals.tracks.them.status = TrackStatus::Live;
    assert!(rig.ticks(4, &[Lane::Me]).is_empty(), "the window restarts");
    assert_eq!(rig.tick(&[Lane::Me]), [no_audio(Channel::System)]);
}

#[test]
fn speech_without_a_transcript_for_thirty_seconds_is_reported() {
    let mut rig = Rig::new();
    rig.vitals.unanswered = Millis(29_999);
    assert!(rig.tick(&Lane::ALL).is_empty());

    rig.vitals.unanswered = Millis(30_000);
    let expected = issue(IssueKind::NoTranscript, Some(Channel::Mic));
    assert_eq!(rig.tick(&Lane::ALL), [expected]);

    rig.vitals.unanswered = Millis::ZERO;
    assert!(rig.tick(&Lane::ALL).is_empty());
}

#[test]
fn a_crashed_engine_warns_until_its_stream_has_run_again_for_a_while() {
    let mut rig = Rig::new();
    rig.vitals.engines.them = Some(LaneStatus {
        recoveries: 1,
        streaming: true,
    });
    let stopped = issue(IssueKind::TranscriptionStopped, Some(Channel::System));

    assert_eq!(rig.tick(&Lane::ALL), [stopped]);
    assert_eq!(rig.ticks(9, &Lane::ALL), [stopped]);
    assert!(rig.tick(&Lane::ALL).is_empty());
}

#[test]
fn an_engine_without_a_stream_keeps_warning() {
    let mut rig = Rig::new();
    rig.vitals.engines.me = Some(LaneStatus {
        recoveries: 1,
        streaming: false,
    });

    let issues = rig.ticks(60, &Lane::ALL);

    assert_eq!(
        issues,
        [issue(IssueKind::TranscriptionStopped, Some(Channel::Mic))]
    );
}

#[test]
fn low_disk_warns_with_hysteresis_and_a_full_disk_stops() {
    let mut rig = Rig::new();
    let low = issue(IssueKind::DiskLow, None);
    rig.vitals.free_disk = Some(Bytes::gib(1));
    assert_eq!(rig.tick(&Lane::ALL), [low]);

    rig.vitals.free_disk = Some(Bytes(DISK_LOW.0 + 1));
    assert_eq!(rig.tick(&Lane::ALL), [low], "inside the margin");
    rig.vitals.free_disk = Some(Bytes(DISK_LOW.0 + DISK_MARGIN.0));
    assert!(rig.tick(&Lane::ALL).is_empty());

    rig.vitals.free_disk = Some(Bytes(DISK_FULL.0 - 1));
    assert_eq!(rig.tick(&Lane::ALL), [issue(IssueKind::DiskFull, None)]);
}

#[test]
fn issues_are_reported_once_per_change() {
    let mut rig = Rig::new();
    rig.vitals.free_disk = Some(Bytes::gib(1));
    rig.now = rig.now.plus(SECOND);
    assert!(rig.monitor.evaluate(&rig.vitals, rig.now));
    rig.now = rig.now.plus(SECOND);
    rig.vitals.tracks.me.frames += Frames(48_000);
    rig.vitals.tracks.them.frames += Frames(48_000);
    assert!(!rig.monitor.evaluate(&rig.vitals, rig.now));
}
