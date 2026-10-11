//! Measures the system-audio tap on real hardware (macOS only).
//!
//! ```bash
//! cargo run -p deino-audio --example tap_probe -- 30
//! ```
//!
//! Starts the same `SystemTap` the app records with and prints, once a
//! second: the frames delivered, the loudest sample, whether the aggregate
//! device is alive, and the current default output device. Use it to check
//! (a) that the IO cycle keeps running in silence, and (b) whether the tap
//! keeps capturing across an output route change (plug in headphones,
//! connect `AirPods`, switch the output in Control Centre mid-run). ADR 0026
//! records the results.
#![allow(
    clippy::print_stdout,
    clippy::unwrap_used,
    clippy::cast_precision_loss,
    reason = "a manual measurement tool that reports on stdout"
)]

#[cfg(target_os = "macos")]
fn main() {
    probe::run();
}

#[cfg(not(target_os = "macos"))]
fn main() {}

#[cfg(target_os = "macos")]
mod probe {
    use std::sync::mpsc::channel;
    use std::time::Duration;

    use cidre::core_audio as ca;
    use deino_audio::AudioSource;
    use deino_audio::capture::macos::SystemTap;
    use deino_audio::capture::meter::peak_of;

    /// Plays a quiet 440 Hz tone on the named output device (not
    /// necessarily the default) until the returned stream drops.
    fn tone_on(device_name: &str) -> cpal::Stream {
        use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
        let host = cpal::default_host();
        let device = host
            .output_devices()
            .unwrap()
            .find(|d| d.name().is_ok_and(|n| n == device_name))
            .unwrap();
        let config = device.default_output_config().unwrap();
        let rate = config.sample_rate().0 as f32;
        let channels = usize::from(config.channels());
        let mut phase = 0.0_f32;
        let stream = device
            .build_output_stream(
                &config.config(),
                move |data: &mut [f32], _| {
                    for frame in data.chunks_mut(channels) {
                        phase = (phase + 440.0 / rate) % 1.0;
                        frame.fill((phase * std::f32::consts::TAU).sin() * 0.05);
                    }
                },
                |_| {},
                None,
            )
            .unwrap();
        stream.play().unwrap();
        stream
    }

    /// `tap_probe tone <device> <seconds>`: only plays the tone. Run it as a
    /// second process, because the tap excludes the probe's own process.
    fn tone_only(device: &str, seconds: u64) {
        let _tone = tone_on(device);
        let (_keep, timer) = channel::<()>();
        timer.recv_timeout(Duration::from_secs(seconds)).ok();
    }

    pub fn run() {
        let args: Vec<String> = std::env::args().collect();
        if let [_, mode, device, seconds] = args.as_slice()
            && mode == "tone"
        {
            return tone_only(device, seconds.parse().unwrap_or(3));
        }
        let seconds: u64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(10);
        let mut tap = SystemTap::new();
        let mut stream = tap.start().unwrap();
        println!("tap started at {}", stream.rate);
        let (_keep, timer) = channel::<()>();
        let mut block = Vec::new();
        for second in 1..=seconds {
            timer.recv_timeout(Duration::from_secs(1)).ok();
            let (mut frames, mut blocks, mut peak) = (0, 0, 0.0_f32);
            while stream.blocks.pop_block(&mut block).is_some() {
                frames += block.len();
                blocks += 1;
                peak = peak.max(peak_of(&block));
            }
            let output = ca::System::default_output_device()
                .and_then(|device| device.name())
                .map(|name| name.to_string())
                .unwrap_or_default();
            println!(
                "{second:>3}s frames={frames:>6} callbacks={blocks:>4} peak={peak:.4} alive={} output={output:?}",
                tap.is_alive()
            );
        }
        tap.stop();
    }
}
