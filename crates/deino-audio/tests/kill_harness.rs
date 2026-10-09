//! FND-05 acceptance on real hardware timing (ignored by default; run with
//! `cargo test -p deino-audio --test kill_harness -- --ignored --nocapture
//! --test-threads=1`):
//!
//! - `kill_minus_nine_loses_at_most_one_second`: a child process records the
//!   mock meeting in real time through the real encoder; the parent sends it
//!   `SIGKILL` at a random moment, runs launch recovery, reads both tracks
//!   with `AVAudioFile` and checks that at most 1 s was lost.
//! - `cpu_per_track_is_about_two_percent`: records 20 s in real time and
//!   reports the process CPU per track, with the machine load next to it.
#![cfg(target_os = "macos")]
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "an integration-test crate: helpers fail the test by panicking, like #[test] bodies"
)]

#[path = "support/decode.rs"]
mod decode;

use std::path::Path;
use std::process::{Child, Command};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Duration;

use decode::{decode_with_av_audio_file, seconds};
use deino_audio::recovery::Recovery;
use deino_audio::{
    AudioSource, CmafEncoderFactory, Frames, HostClock, MockAudioSource, Recorder, RecorderDeps,
    SystemDiskSpace, SystemHostClock, Track, recover_recording,
};
use deino_domain::SystemClock;

/// Tells the child test where to record; without it the child does nothing.
const CHILD_DIR_ENV: &str = "DEINO_KILL_HARNESS_DIR";
/// The child's progress file: frames each writer has handed to its encoder.
const PROGRESS_FILE: &str = "progress.txt";
/// How often the child reports progress.
const PROGRESS_EVERY: Duration = Duration::from_millis(20);
/// Kill trials per run.
const TRIALS: u64 = 3;
/// The FND-05 bound on audio lost to `kill -9`.
const MAX_LOSS_SECONDS: f64 = 1.0;

/// A timer to wait on: a channel that never receives.
struct Ticker {
    _keep: Sender<()>,
    ticks: Receiver<()>,
}

impl Ticker {
    fn new() -> Self {
        let (keep, ticks) = channel();
        Self { _keep: keep, ticks }
    }

    fn wait(&self, duration: Duration) {
        assert!(self.ticks.recv_timeout(duration).is_err());
    }
}

/// Real adapters, as the app will wire them.
fn real_deps(clock: &Arc<dyn HostClock>) -> RecorderDeps {
    RecorderDeps {
        encoders: Arc::new(CmafEncoderFactory),
        host_clock: Arc::clone(clock),
        wall_clock: Arc::new(SystemClock),
        disk: Arc::new(SystemDiskSpace),
    }
}

/// Starts recording the mock meeting into `dir`.
fn start_meeting(dir: &Path) -> (Recorder, [MockAudioSource; 2]) {
    let clock: Arc<dyn HostClock> = Arc::new(SystemHostClock::new());
    let (recorder, inputs) = Recorder::start(dir, &real_deps(&clock)).unwrap();
    let [mut microphone, mut system] = MockAudioSource::meeting(&clock);
    microphone.start(inputs.microphone).unwrap();
    system.start(inputs.system).unwrap();
    (recorder, [microphone, system])
}

#[test]
#[ignore = "child process of kill_minus_nine_loses_at_most_one_second"]
fn kill_child_records_until_killed() {
    let Ok(dir) = std::env::var(CHILD_DIR_ENV) else {
        return;
    };
    let dir = Path::new(&dir);
    let (recorder, _sources) = start_meeting(dir);
    let ticker = Ticker::new();
    loop {
        let line = format!(
            "{} {}\n",
            recorder.appended(Track::Microphone).0,
            recorder.appended(Track::System).0
        );
        std::fs::write(dir.join("progress.tmp"), line).unwrap();
        std::fs::rename(dir.join("progress.tmp"), dir.join(PROGRESS_FILE)).unwrap();
        ticker.wait(PROGRESS_EVERY);
    }
}

/// The child's last reported progress, per track.
fn progress(dir: &Path) -> Option<[u64; 2]> {
    let text = std::fs::read_to_string(dir.join(PROGRESS_FILE)).ok()?;
    let mut numbers = text.split_whitespace().map(|n| n.parse().unwrap());
    Some([numbers.next()?, numbers.next()?])
}

/// Spawns this test binary as the recording child.
fn spawn_child(dir: &Path) -> Child {
    Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "kill_child_records_until_killed",
            "--ignored",
            "--nocapture",
        ])
        .env(CHILD_DIR_ENV, dir)
        .spawn()
        .unwrap()
}

/// A kill time in [3, 6) s, varied per trial.
fn kill_after(trial: u64) -> Frames {
    let seed = SystemHostClock::new().now().0 ^ (trial.wrapping_mul(0x9E37_79B9_7F4A_7C15));
    Frames(3 * 48_000 + seed % (3 * 48_000))
}

#[test]
#[ignore = "real-time acceptance harness; about 20 s"]
fn kill_minus_nine_loses_at_most_one_second() {
    let ticker = Ticker::new();
    for trial in 0..TRIALS {
        let dir = tempfile::tempdir().unwrap();
        let mut child = spawn_child(dir.path());
        let target = kill_after(trial);
        while progress(dir.path()).is_none_or(|[mic, _]| mic < target.0) {
            ticker.wait(Duration::from_millis(5));
        }
        child.kill().unwrap();
        child.wait().unwrap();
        let appended = progress(dir.path()).unwrap();

        let Recovery::Recovered { manifest, .. } = recover_recording(dir.path()).unwrap() else {
            panic!("a killed recording needs recovery");
        };
        for (track, appended) in Track::ALL.into_iter().zip(appended) {
            let entry = manifest.track(track).unwrap();
            let file = dir.path().join(entry.segments[0].relative_path(track));
            let decoded = Frames::of_len(decode_with_av_audio_file(&file).len());
            let (appended, decoded) = (seconds(appended), seconds(decoded.0));
            let loss = appended - decoded;
            println!(
                "trial {trial} {track}: appended {appended:.3} s, decoded {decoded:.3} s, lost {loss:.3} s"
            );
            assert!(decoded > 0.0, "{track} is readable by AVAudioFile");
            assert!(loss <= MAX_LOSS_SECONDS, "{track} lost {loss:.3} s");
        }
    }
}

/// User + system CPU time of this process.
#[allow(unsafe_code, reason = "getrusage fills a zeroed struct we own")]
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
#[allow(unsafe_code, reason = "getloadavg fills an array we own")]
fn load_average() -> f64 {
    let mut loads = [0.0_f64; 3];
    // SAFETY: the pointer and count describe `loads`.
    unsafe { libc::getloadavg(loads.as_mut_ptr(), 3) };
    loads[0]
}

#[test]
#[ignore = "real-time measurement; about 20 s"]
fn cpu_per_track_is_about_two_percent() {
    let dir = tempfile::tempdir().unwrap();
    let wall = SystemHostClock::new();
    let (cpu_before, started) = (cpu_time(), wall.now());
    let (recorder, mut sources) = start_meeting(dir.path());
    Ticker::new().wait(Duration::from_secs(20));
    sources.iter_mut().for_each(AudioSource::stop);
    recorder.stop().unwrap();
    let cpu = cpu_time().saturating_sub(cpu_before).as_secs_f64();
    let elapsed = wall.now().since(started).as_secs_f64();
    let per_track = 100.0 * cpu / elapsed / 2.0;
    let load = load_average();
    println!("CPU per track: {per_track:.2} % of one core (load average {load:.1})");
    if load < 20.0 {
        assert!(per_track <= 3.0, "{per_track:.2} % per track");
    }
}
