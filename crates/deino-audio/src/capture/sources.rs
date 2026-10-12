//! `DeviceSources`: the real devices behind a recording.
//!
//! The microphone is cpal on every platform ([`MicSource`]); system audio
//! is the Core Audio process tap on macOS and unavailable elsewhere (the
//! recording then has a silent system track, §1 D1).

use std::sync::Arc;

use super::devices::MicChoice;
use super::mic::{CpalInputs, MicSource};
use crate::clock::HostClock;
use crate::source::{AudioSource, SourceError, SourceFactory, SourceStream};
use crate::units::Track;

/// The cpal microphone and, on macOS, the process tap.
#[derive(Debug)]
pub struct DeviceSources {
    choice: MicChoice,
    clock: Arc<dyn HostClock>,
}

impl DeviceSources {
    /// Sources following the microphone `choice`.
    pub fn new(choice: MicChoice, clock: Arc<dyn HostClock>) -> Self {
        Self { choice, clock }
    }
}

impl SourceFactory for DeviceSources {
    fn microphone(&self) -> Box<dyn AudioSource> {
        self.microphone_for(&self.choice)
    }

    fn microphone_for(&self, choice: &MicChoice) -> Box<dyn AudioSource> {
        let devices = Arc::new(CpalInputs);
        Box::new(MicSource::new(
            choice.clone(),
            devices,
            Arc::clone(&self.clock),
        ))
    }

    #[cfg(target_os = "macos")]
    fn system(&self) -> Box<dyn AudioSource> {
        Box::new(super::macos::SystemTap::new())
    }

    #[cfg(not(target_os = "macos"))]
    fn system(&self) -> Box<dyn AudioSource> {
        Box::new(Unavailable(Track::System))
    }
}

/// A track this platform cannot capture: starting it always fails with
/// [`SourceError::NoDevice`], so the track records silence.
#[derive(Clone, Copy, Debug)]
pub struct Unavailable(pub Track);

impl AudioSource for Unavailable {
    fn track(&self) -> Track {
        self.0
    }

    fn start(&mut self) -> Result<SourceStream, SourceError> {
        Err(SourceError::NoDevice(self.0))
    }

    fn stop(&mut self) {}

    fn is_alive(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unavailable_track_never_starts() {
        let mut source = Unavailable(Track::System);
        assert!(matches!(
            source.start(),
            Err(SourceError::NoDevice(Track::System))
        ));
        assert!(!source.is_alive());
    }
}
