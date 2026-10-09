//! How long worker calls may take before the worker counts as hung.
//!
//! Values from Handy's supervisor (MIT), where they were tuned on real
//! hardware. A hang on the GPU is recovered (retried on the CPU), so it is
//! caught early; on the CPU it is never retried, so a slow machine must not be
//! mistaken for a hung one.

use std::time::Duration;

use crate::pcm::Millis;

/// Deadlines for decoding work, by the device the model runs on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Deadlines {
    /// Floor for a stream feed or finalize, so sub-second chunks and a
    /// first-use shader compile are not mistaken for hangs.
    pub call_floor: Duration,
    /// Floor for a one-shot run, which has a fixed cost however short.
    pub run_floor: Duration,
    /// Work on N seconds of audio may take up to this many times N.
    pub audio_factor: u32,
}

impl Deadlines {
    /// Deadlines on a GPU.
    pub const GPU: Self = Self {
        call_floor: Duration::from_secs(30),
        run_floor: Duration::from_mins(1),
        audio_factor: 10,
    };

    /// Deadlines on the CPU, which can run slower than real time.
    pub const CPU: Self = Self {
        call_floor: Duration::from_mins(2),
        run_floor: Duration::from_mins(5),
        audio_factor: 20,
    };

    /// The deadline for stream work on `audio`.
    pub fn stream(self, audio: Millis) -> Duration {
        let scaled = Duration::from_millis(audio.0).saturating_mul(self.audio_factor);
        self.call_floor.max(scaled)
    }

    /// The deadline for a one-shot run on `audio`.
    pub fn run(self, audio: Millis) -> Duration {
        self.run_floor.max(self.stream(audio))
    }
}

/// Supervisor settings. [`SupervisorConfig::default`] is what the app uses;
/// tests shorten the deadlines.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SupervisorConfig {
    /// Whether this host may use a GPU at all. When not, every worker is
    /// CPU-only and no GPU driver is ever loaded.
    pub gpu_allowed: bool,
    /// Spawn plus backend init. Opening the GPU compiles ggml's Metal library
    /// when the system shader cache lacks it (about 15 s after an update).
    pub hello: Duration,
    /// A model load: slow is not hung.
    pub load: Duration,
    /// Stream calls that decode nothing (begin, reset).
    pub control: Duration,
    /// How long a worker gets to exit after its stdin closes.
    pub shutdown_grace: Duration,
    /// Decoding deadlines on a GPU.
    pub gpu: Deadlines,
    /// Decoding deadlines on the CPU.
    pub cpu: Deadlines,
}

impl Default for SupervisorConfig {
    fn default() -> Self {
        Self {
            gpu_allowed: true,
            hello: Duration::from_mins(1),
            load: Duration::from_mins(3),
            control: Duration::from_secs(30),
            shutdown_grace: Duration::from_secs(2),
            gpu: Deadlines::GPU,
            cpu: Deadlines::CPU,
        }
    }
}

impl SupervisorConfig {
    /// The decoding deadlines for a model on the GPU or the CPU.
    pub fn deadlines(&self, on_gpu: bool) -> Deadlines {
        if on_gpu { self.gpu } else { self.cpu }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deadlines_scale_with_the_audio_above_a_floor() {
        let gpu = Deadlines::GPU;

        assert_eq!(gpu.stream(Millis(500)), gpu.call_floor);
        assert_eq!(gpu.stream(Millis(5000)), Duration::from_secs(50));
        assert_eq!(gpu.run(Millis(1000)), gpu.run_floor);
        assert_eq!(gpu.run(Millis(10_000)), Duration::from_secs(100));
    }

    #[test]
    fn the_cpu_is_never_held_to_a_stricter_deadline_than_the_gpu() {
        for seconds in [0, 1, 5, 30, 120, 600] {
            let audio = Millis(seconds * 1000);
            assert!(Deadlines::CPU.stream(audio) >= Deadlines::GPU.stream(audio));
            assert!(Deadlines::CPU.run(audio) >= Deadlines::GPU.run(audio));
        }
    }
}
