//! [`CoreAudioProcesses`]: every Core Audio client and whether it captures or
//! plays, from `kAudioHardwarePropertyProcessObjectList` and each process
//! object's `IsRunningInput`/`IsRunningOutput`/`BundleID` (§3C; not `ps`).
//!
//! The watch listens to the process list and to both running flags of every
//! process object, re-attaching per-object listeners whenever the list
//! changes (OpenWhispr `macos-mic-listener.swift`, ported to Rust). A process
//! that vanishes between listing and reading is skipped, not an error.

use std::collections::HashMap;

use cidre::core_audio as ca;
use deino_domain::BundleId;

use super::listener::{CaListener, status};
use crate::error::DetectError;
use crate::model::{AudioProcess, Pid};
use crate::ports::{Sensor, Waker, Watch};

/// The Core Audio process objects.
pub(super) struct CoreAudioProcesses;

impl Sensor<Vec<AudioProcess>> for CoreAudioProcesses {
    fn read(&self) -> Result<Vec<AudioProcess>, DetectError> {
        let processes = ca::System::processes()
            .map_err(|error| DetectError::os("the audio process list", status(error)))?;
        Ok(processes.iter().filter_map(read_process).collect())
    }

    fn watch(&self, waker: &Waker) -> Result<Box<dyn Watch>, DetectError> {
        let list = CaListener::add(
            *ca::System::OBJ,
            ca::PropSelector::HW_PROCESS_OBJ_LIST,
            waker,
            "the audio process list",
        )?;
        let mut watch = ProcessWatch {
            waker: waker.clone(),
            _list: list,
            objects: HashMap::new(),
        };
        watch.refresh();
        Ok(Box::new(watch))
    }
}

/// One process object, or `None` if it exited while being read.
fn read_process(process: &ca::Process) -> Option<AudioProcess> {
    let pid = process.pid().ok()?;
    let running_input = process.is_running_input().ok()?;
    let running_output = process.is_running_output().unwrap_or(false);
    let bundle_id = process
        .bundle_id()
        .ok()
        .and_then(|id| BundleId::try_from(id.to_string()).ok());
    Some(AudioProcess {
        pid: Pid(pid),
        bundle_id,
        running_input,
        running_output,
    })
}

/// Listeners on the process list and on each process object's running flags.
struct ProcessWatch {
    waker: Waker,
    _list: CaListener,
    objects: HashMap<u32, [CaListener; 2]>,
}

impl Watch for ProcessWatch {
    fn refresh(&mut self) {
        let Ok(processes) = ca::System::processes() else {
            return;
        };
        let current: Vec<ca::Obj> = processes.iter().map(|process| process.0).collect();
        self.objects
            .retain(|id, _| current.iter().any(|object| object.0 == *id));
        for object in current {
            if self.objects.contains_key(&object.0) {
                continue;
            }
            // A process can exit before its listeners register; the next
            // refresh retries any object that is still listed.
            if let Ok(listeners) = running_listeners(object, &self.waker) {
                self.objects.insert(object.0, listeners);
            }
        }
    }
}

fn running_listeners(object: ca::Obj, waker: &Waker) -> Result<[CaListener; 2], DetectError> {
    let input = CaListener::add(
        object,
        ca::PropSelector::PROCESS_IS_RUNNING_INPUT,
        waker,
        "a process's input state",
    )?;
    let output = CaListener::add(
        object,
        ca::PropSelector::PROCESS_IS_RUNNING_OUTPUT,
        waker,
        "a process's output state",
    )?;
    Ok([input, output])
}
