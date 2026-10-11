//! `MicSource`: the microphone through cpal.
//!
//! Each start ranks the input devices ([`super::devices::rank`]) and opens
//! the first that works. A stream is built, played and dropped on its own
//! thread, which reports back within [`OPEN_TIMEOUT`] (anarlog's mic thread):
//! a driver that hangs in `build_input_stream` cannot hang the recording.
//! The cpal error callback only clears the `alive` flag; the supervisor
//! notices and restarts the source. Never opened exclusively (§3C).

mod stream;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::thread::JoinHandle;
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait};

use super::devices::{InputDevice, InputDevices, MicChoice, Transport, is_hands_free, rank};
use crate::clock::HostClock;
use crate::source::{AudioSource, SourceError, SourceStream};
use crate::units::Track;

/// How long a device may take to start before the next one is tried.
pub const OPEN_TIMEOUT: Duration = Duration::from_secs(5);

/// A running stream's thread.
#[derive(Debug)]
struct Running {
    stop: Sender<()>,
    thread: JoinHandle<()>,
    transport: Transport,
    rate: u32,
}

/// The microphone source.
#[derive(Debug)]
pub struct MicSource {
    choice: MicChoice,
    devices: Arc<dyn InputDevices>,
    clock: Arc<dyn HostClock>,
    alive: Arc<AtomicBool>,
    running: Option<Running>,
}

impl MicSource {
    /// A microphone following `choice`, picked among `devices`.
    pub fn new(
        choice: MicChoice,
        devices: Arc<dyn InputDevices>,
        clock: Arc<dyn HostClock>,
    ) -> Self {
        Self {
            choice,
            devices,
            clock,
            alive: Arc::new(AtomicBool::new(false)),
            running: None,
        }
    }

    /// Opens the best device that works.
    fn open_best(&mut self) -> Result<SourceStream, SourceError> {
        let devices = self.devices.inputs();
        let mut last = SourceError::NoDevice(Track::Microphone);
        for device in rank(&self.choice, &devices) {
            match self.open(device) {
                Ok(stream) => return Ok(stream),
                Err(error) => {
                    log::warn!(target: "deino_audio::capture", "microphone {:?} did not open: {error}", device.name);
                    last = error;
                }
            }
        }
        Err(last)
    }

    fn open(&mut self, device: &InputDevice) -> Result<SourceStream, SourceError> {
        self.alive.store(true, Ordering::Release);
        let opened = stream::open(stream::OpenRequest {
            name: device.name.clone(),
            clock: Arc::clone(&self.clock),
            alive: Arc::clone(&self.alive),
        })?;
        log::info!(target: "deino_audio::capture", "microphone {:?} at {}", device.name, opened.stream.rate);
        self.running = Some(Running {
            stop: opened.stop,
            thread: opened.thread,
            transport: device.transport,
            rate: opened.stream.rate.0,
        });
        Ok(opened.stream)
    }
}

impl AudioSource for MicSource {
    fn track(&self) -> Track {
        Track::Microphone
    }

    fn start(&mut self) -> Result<SourceStream, SourceError> {
        if self.running.is_some() {
            return Err(SourceError::AlreadyStarted(Track::Microphone));
        }
        self.open_best()
    }

    fn stop(&mut self) {
        let Some(running) = self.running.take() else {
            return;
        };
        running.stop.send(()).ok();
        if running.thread.join().is_err() {
            log::error!(target: "deino_audio::capture", "the microphone thread panicked");
        }
    }

    fn is_alive(&self) -> bool {
        self.running.is_some() && self.alive.load(Ordering::Acquire)
    }

    fn hands_free(&self) -> bool {
        self.running
            .as_ref()
            .is_some_and(|running| is_hands_free(running.transport, running.rate))
    }
}

impl Drop for MicSource {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Lists inputs through cpal; on macOS the transport comes from Core Audio.
#[derive(Clone, Copy, Debug, Default)]
pub struct CpalInputs;

impl InputDevices for CpalInputs {
    fn inputs(&self) -> Vec<InputDevice> {
        let host = cpal::default_host();
        let default = host
            .default_input_device()
            .and_then(|device| device.name().ok());
        let Ok(devices) = host.input_devices() else {
            return Vec::new();
        };
        devices
            .filter_map(|device| device.name().ok())
            .map(|name| InputDevice {
                transport: transport_of(&name),
                is_default: default.as_deref() == Some(name.as_str()),
                name,
            })
            .collect()
    }
}

#[cfg(target_os = "macos")]
fn transport_of(name: &str) -> Transport {
    super::macos::devices::transport_of(name)
}

#[cfg(not(target_os = "macos"))]
fn transport_of(_name: &str) -> Transport {
    Transport::Other
}
