//! Core Audio device facts: transports, headphone detection, and the
//! output route probe.
//!
//! Headphone detection follows anarlog's `audio-device` crate (MIT): a
//! stream terminal type of headphones or headset decides; otherwise the
//! transport and name do ([`route::headphone_by_name`]). The route counts
//! only devices *other* processes play to, so Deino's own sounds never
//! decide it.

use std::collections::HashSet;

use cidre::core_audio as ca;

use crate::capture::devices::Transport;
use crate::capture::route::{self, OutputDevice, OutputRoute, RouteProbe};

/// The output device list each process renders to.
const PROCESS_OUTPUT_DEVICES: ca::PropAddr = ca::PropAddr {
    selector: ca::PropSelector::PROCESS_DEVICES,
    scope: ca::PropScope::OUTPUT,
    element: ca::PropElement::MAIN,
};

/// The transport of the device named `name`, or `Other` when unknown.
pub fn transport_of(name: &str) -> Transport {
    ca::System::devices()
        .unwrap_or_default()
        .into_iter()
        .find(|device| device.name().is_ok_and(|n| n.to_string() == name))
        .map_or(Transport::Other, |device| transport(&device))
}

/// A device's transport.
pub fn transport(device: &ca::Device) -> Transport {
    use ca::DeviceTransportType as Kind;
    let Ok(kind) = device.transport_type() else {
        return Transport::Other;
    };
    match kind.0 {
        k if k == Kind::BUILT_IN.0 => Transport::BuiltIn,
        k if k == Kind::USB.0 => Transport::Usb,
        k if k == Kind::BLUETOOTH.0 || k == Kind::BLUETOOTH_LE.0 => Transport::Bluetooth,
        k if k == Kind::VIRTUAL.0 || k == Kind::AGGREGATE.0 => Transport::Virtual,
        _ => Transport::Other,
    }
}

/// Whether `device` is a headphone.
fn is_headphone(device: &ca::Device, name: &str) -> bool {
    let terminal = device.streams().unwrap_or_default().iter().any(|stream| {
        stream.terminal_type().is_ok_and(|kind| {
            kind.0 == ca::StreamTerminalType::HEADPHONES.0
                || kind.0 == ca::StreamTerminalType::HEADSET_MIC.0
        })
    });
    terminal || route::headphone_by_name(name, transport(device) == Transport::Bluetooth)
}

fn output_device(device: &ca::Device) -> OutputDevice {
    let name = device.name().map(|n| n.to_string()).unwrap_or_default();
    OutputDevice {
        is_headphone: is_headphone(device, &name),
        name,
    }
}

/// Object IDs of the devices other processes are playing to.
fn foreign_playing_devices() -> HashSet<u32> {
    let own = i32::try_from(std::process::id()).unwrap_or_default();
    ca::System::processes()
        .unwrap_or_default()
        .iter()
        .filter(|process| process.pid().is_ok_and(|pid| pid != own))
        .filter(|process| process.is_running_output().unwrap_or(false))
        .filter_map(|process| process.prop_vec::<ca::Device>(&PROCESS_OUTPUT_DEVICES).ok())
        .flatten()
        .map(|device| device.0.0)
        .collect()
}

/// The route probe over Core Audio.
#[derive(Clone, Copy, Debug, Default)]
pub struct CoreAudioRoute;

impl RouteProbe for CoreAudioRoute {
    fn route(&self) -> Option<OutputRoute> {
        let default = ca::System::default_output_device()
            .ok()
            .map(|d| output_device(&d));
        let playing_ids = foreign_playing_devices();
        let playing: Vec<OutputDevice> = ca::System::devices()
            .unwrap_or_default()
            .iter()
            .filter(|device| playing_ids.contains(&device.0.0))
            .map(output_device)
            .filter(|device| !crate::capture::devices::is_loopback(&device.name))
            .collect();
        route::verdict(default.as_ref(), &playing)
    }
}
