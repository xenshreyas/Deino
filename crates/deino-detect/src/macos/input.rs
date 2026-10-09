//! [`DefaultInput`]: whether the default input device is running for any
//! process (`kAudioDevicePropertyDeviceIsRunningSomewhere`).
//!
//! It catches capture that process attribution cannot name. The watch
//! follows default-input changes (`kAudioHardwarePropertyDefaultInputDevice`)
//! and re-arms on the new device (anarlog `crates/detect/src/mic/macos`).

use cidre::core_audio as ca;

use super::listener::{CaListener, status};
use crate::error::DetectError;
use crate::ports::{Sensor, Waker, Watch};

/// The default input device.
pub(super) struct DefaultInput;

impl Sensor<bool> for DefaultInput {
    fn read(&self) -> Result<bool, DetectError> {
        let device = ca::System::default_input_device()
            .map_err(|error| DetectError::os("the default input device", status(error)))?;
        device
            .bool_prop(&ca::PropSelector::DEVICE_IS_RUNNING_SOMEWHERE.global_addr())
            .map_err(|error| DetectError::os("whether the microphone runs", status(error)))
    }

    fn watch(&self, waker: &Waker) -> Result<Box<dyn Watch>, DetectError> {
        let default = CaListener::add(
            *ca::System::OBJ,
            ca::PropSelector::HW_DEFAULT_INPUT_DEVICE,
            waker,
            "the default input device",
        )?;
        let mut watch = InputWatch {
            waker: waker.clone(),
            _default: default,
            device: None,
        };
        watch.refresh();
        Ok(Box::new(watch))
    }
}

/// Listens for default-input changes and to the current device.
struct InputWatch {
    waker: Waker,
    _default: CaListener,
    device: Option<(u32, CaListener)>,
}

impl Watch for InputWatch {
    fn refresh(&mut self) {
        let Ok(device) = ca::System::default_input_device() else {
            self.device = None;
            return;
        };
        if self
            .device
            .as_ref()
            .is_some_and(|(id, _)| *id == device.0.0)
        {
            return;
        }
        let listener = CaListener::add(
            device.0,
            ca::PropSelector::DEVICE_IS_RUNNING_SOMEWHERE,
            &self.waker,
            "whether the microphone runs",
        );
        self.device = listener.ok().map(|listener| (device.0.0, listener));
    }
}
