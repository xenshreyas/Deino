//! [`Cameras`]: whether any camera is running for any process
//! (`kCMIODevicePropertyDeviceIsRunningSomewhere`), with listeners on the
//! device list (hot-plug) and on every camera (Muesli's camera observer).
//!
//! The camera only strengthens microphone use; it never prompts on its own
//! (Photo Booth, document scanning).

use std::collections::HashMap;

use objc2_core_media_io::CMIOObjectID;

use super::cmio::{self, CmioListener, DEVICES, RUNNING_SOMEWHERE, SYSTEM};
use crate::error::DetectError;
use crate::ports::{Sensor, Waker, Watch};

/// The Mac's cameras.
pub(super) struct Cameras;

impl Sensor<bool> for Cameras {
    fn read(&self) -> Result<bool, DetectError> {
        let cameras =
            cmio::cameras().map_err(|status| DetectError::os("the camera list", status))?;
        Ok(cameras.into_iter().any(cmio::is_running_somewhere))
    }

    fn watch(&self, waker: &Waker) -> Result<Box<dyn Watch>, DetectError> {
        let list = CmioListener::add(SYSTEM, DEVICES, waker)
            .map_err(|status| DetectError::os("the camera list", status))?;
        let mut watch = CameraWatch {
            waker: waker.clone(),
            _list: list,
            cameras: HashMap::new(),
        };
        watch.refresh();
        Ok(Box::new(watch))
    }
}

/// Listeners on the device list and on each camera.
struct CameraWatch {
    waker: Waker,
    _list: CmioListener,
    cameras: HashMap<CMIOObjectID, CmioListener>,
}

impl Watch for CameraWatch {
    fn refresh(&mut self) {
        let Ok(current) = cmio::cameras() else { return };
        self.cameras.retain(|camera, _| current.contains(camera));
        for camera in current {
            if self.cameras.contains_key(&camera) {
                continue;
            }
            // An unplugged camera can fail to register; the next refresh retries.
            if let Ok(listener) = CmioListener::add(camera, RUNNING_SOMEWHERE, &self.waker) {
                self.cameras.insert(camera, listener);
            }
        }
    }
}
