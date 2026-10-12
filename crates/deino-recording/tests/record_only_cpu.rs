//! CAP-08 acceptance: with live transcription off, recording stays under
//! 15 % of one core.
//!
//! `cargo test -p deino-recording --test record_only_cpu -- --ignored --nocapture`
//!
//! Records the one-on-one fixture for [`LENGTH`] in real time through mock
//! capture and the real `AVAssetWriter` encoder (macOS), once record-only
//! and once with live transcription on the scripted fake engine (so the live
//! pipeline's own cost, AEC and VAD, shows; real engines add their worker
//! processes, see ADR 0026), and prints this process's CPU as a share of one
//! core, with the machine's load average beside it. Device callbacks are not
//! included: mock capture stands in for the microphone and the tap.
#![cfg(target_os = "macos")]
#![allow(
    clippy::unwrap_used,
    clippy::print_stdout,
    unsafe_code,
    reason = "an integration-test crate that measures: getrusage and getloadavg are libc calls"
)]

#[path = "support/rig.rs"]
mod rig;

use std::sync::Arc;
use std::time::Duration;

use deino_audio::capture::OutputRoute;
use deino_audio::{CmafEncoderFactory, HostClock, SystemHostClock, TestSignal};
use deino_domain::Channel;
use deino_recording::{Preferences, StartRequest};
use deino_testkit::Fixtures;
use deino_testkit::fixture::{MeetingKind, PcmTrack};
use rig::{FastClock, FixtureCapture, RigOptions, rig_over};

/// How long each recording runs, in real time.
const LENGTH: Duration = Duration::from_secs(30);

/// CAP-08's bound, as a share of one core.
const MAX_RECORD_ONLY_PERCENT: f64 = 15.0;

/// User + system CPU time of this process.
fn cpu_time() -> Duration {
    // SAFETY: `usage` is a valid, writable rusage for the call.
    let usage = unsafe {
        let mut usage: libc::rusage = std::mem::zeroed();
        libc::getrusage(libc::RUSAGE_SELF, &raw mut usage);
        usage
    };
    let time = |t: libc::timeval| {
        let micros =
            u64::try_from(t.tv_sec).unwrap() * 1_000_000 + u64::try_from(t.tv_usec).unwrap();
        Duration::from_micros(micros)
    };
    time(usage.ru_utime) + time(usage.ru_stime)
}

/// The one-minute load average.
fn load_average() -> f64 {
    let mut loads = [0.0_f64; 3];
    // SAFETY: the pointer and count describe `loads`.
    unsafe { libc::getloadavg(loads.as_mut_ptr(), 3) };
    loads[0]
}

fn voices() -> (TestSignal, TestSignal) {
    let fixtures = Fixtures::load().unwrap();
    let meeting = fixtures.meeting(MeetingKind::OneOnOne).unwrap();
    let audio = meeting.audio.as_ref().unwrap();
    let decode = |channel| {
        let track = PcmTrack::decode(&fixtures.audio_path(audio, channel).unwrap()).unwrap();
        TestSignal::from_pcm(&track.samples, track.sample_rate)
    };
    (decode(Channel::Mic), decode(Channel::System))
}

/// Records [`LENGTH`] in real time; returns the CPU used as % of one core.
async fn record(record_only: bool) -> f64 {
    let (microphone, system) = voices();
    let capture = FixtureCapture {
        clock: Arc::new(FastClock::new(1)),
        microphone,
        system,
        route: OutputRoute::Speakers,
    };
    let options = RigOptions {
        preferences: Preferences {
            record_only,
            ..Preferences::default()
        },
        encoders: Arc::new(CmafEncoderFactory),
        ..RigOptions::default()
    };
    let rig = rig_over(capture, options).await;
    let wall = SystemHostClock::new();
    let (cpu_before, started) = (cpu_time(), wall.now());
    rig.service.start(StartRequest::default()).await.unwrap();
    let clock = Arc::clone(&rig.clock);
    tokio::task::spawn_blocking(move || clock.wait_for(LENGTH))
        .await
        .unwrap();
    rig.service.stop().await.unwrap();
    let cpu = cpu_time().saturating_sub(cpu_before).as_secs_f64();
    100.0 * cpu / wall.now().since(started).as_secs_f64()
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "real-time measurement; about a minute"]
async fn record_only_stays_under_fifteen_percent_of_a_core() {
    let record_only = record(true).await;
    let live = record(false).await;
    let load = load_average();

    println!(
        "record-only: {record_only:.1} % of one core; live (fake engine): {live:.1} % (load average {load:.1})"
    );
    assert!(record_only < MAX_RECORD_ONLY_PERCENT, "{record_only:.1} %");
}
