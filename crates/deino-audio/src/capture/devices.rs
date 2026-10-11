//! Which microphone to open: device facts and the ranking rule.
//!
//! The ranking is pure, so it is tested without hardware:
//!
//! 1. the device the user picked, when it is present;
//! 2. when following the system default and the default is a Bluetooth
//!    headset, a wired microphone instead (built-in first, then USB), so
//!    `AirPods` are not forced into the low-quality hands-free (HFP) profile
//!    (anarlog `wired_input_replacing_bluetooth_default`, muesli
//!    `preferredMeetingInputDeviceID`; §3C);
//! 3. the system default;
//! 4. every other input.
//!
//! Deino's own tap device and other loopbacks are never microphones.

/// How a device is connected.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Transport {
    /// The Mac's own microphone or speakers.
    BuiltIn,
    /// USB.
    Usb,
    /// Bluetooth (`AirPods`, headsets).
    Bluetooth,
    /// A virtual or aggregate device (loopbacks, meeting-app drivers).
    Virtual,
    /// Anything else, or unknown.
    Other,
}

/// An input device as the ranking sees it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InputDevice {
    /// The name the system shows (and the settings store).
    pub name: String,
    /// How it is connected.
    pub transport: Transport,
    /// Whether it is the system's default input.
    pub is_default: bool,
}

/// The user's microphone setting.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MicChoice {
    /// The device picked in Settings, by name; `None` follows the system default.
    pub preferred: Option<String>,
}

/// Lists the input devices (cpal names, enriched with Core Audio facts on macOS).
pub trait InputDevices: std::fmt::Debug + Send + Sync {
    /// Every input device, the default flagged.
    fn inputs(&self) -> Vec<InputDevice>;
}

/// Name fragments of devices that capture output, not a person.
const NOT_MICROPHONES: [&str; 4] = [
    "deino system audio",
    "deino-audio-tap",
    "meetily-audio-tap",
    "monitor of ",
];

/// Whether `name` is a loopback or Deino's own tap rather than a microphone.
pub fn is_loopback(name: &str) -> bool {
    let lower = name.to_lowercase();
    NOT_MICROPHONES
        .iter()
        .any(|fragment| lower.contains(fragment))
}

/// The devices to try, best first.
pub fn rank<'a>(choice: &MicChoice, devices: &'a [InputDevice]) -> Vec<&'a InputDevice> {
    let usable: Vec<&InputDevice> = devices.iter().filter(|d| !is_loopback(&d.name)).collect();
    let mut ranked: Vec<&InputDevice> = Vec::with_capacity(usable.len());
    if let Some(name) = choice.preferred.as_deref() {
        ranked.extend(usable.iter().copied().filter(|d| d.name == name));
    }
    let default = usable.iter().copied().find(|d| d.is_default);
    if choice.preferred.is_none() && default.is_some_and(|d| d.transport == Transport::Bluetooth) {
        ranked.extend(wired(&usable));
    }
    ranked.extend(default);
    ranked.extend(usable.iter().copied());
    dedup(ranked)
}

/// Wired microphones, built-in first, then USB.
fn wired<'a>(devices: &[&'a InputDevice]) -> Vec<&'a InputDevice> {
    let of = |transport| {
        devices
            .iter()
            .copied()
            .filter(move |d| d.transport == transport)
    };
    of(Transport::BuiltIn).chain(of(Transport::Usb)).collect()
}

/// Keeps the first occurrence of each device.
fn dedup(devices: Vec<&InputDevice>) -> Vec<&InputDevice> {
    let mut unique: Vec<&InputDevice> = Vec::with_capacity(devices.len());
    for device in devices {
        if !unique.iter().any(|kept| kept.name == device.name) {
            unique.push(device);
        }
    }
    unique
}

/// Below this rate a Bluetooth microphone is in the hands-free (call)
/// profile, which halves speech quality (HFP runs at 8 or 16 kHz).
pub const HANDS_FREE_MAX_RATE: u32 = 24_000;

/// Whether an opened microphone is a Bluetooth headset in call mode.
pub fn is_hands_free(transport: Transport, rate: u32) -> bool {
    transport == Transport::Bluetooth && rate <= HANDS_FREE_MAX_RATE
}

#[cfg(test)]
mod tests {
    use super::*;

    fn device(name: &str, transport: Transport, is_default: bool) -> InputDevice {
        InputDevice {
            name: name.into(),
            transport,
            is_default,
        }
    }

    fn names(ranked: &[&InputDevice]) -> Vec<String> {
        ranked.iter().map(|d| d.name.clone()).collect()
    }

    fn room() -> Vec<InputDevice> {
        vec![
            device("USB Mic", Transport::Usb, false),
            device("AirPods Pro", Transport::Bluetooth, true),
            device("MacBook Pro Microphone", Transport::BuiltIn, false),
            device("Deino System Audio", Transport::Virtual, false),
        ]
    }

    #[test]
    fn a_bluetooth_default_gives_way_to_the_built_in_mic_then_usb() {
        let devices = room();
        let ranked = rank(&MicChoice::default(), &devices);
        assert_eq!(
            names(&ranked),
            ["MacBook Pro Microphone", "USB Mic", "AirPods Pro"]
        );
    }

    #[test]
    fn a_picked_device_wins_even_over_the_bluetooth_rule() {
        let choice = MicChoice {
            preferred: Some("AirPods Pro".into()),
        };
        let devices = room();
        let ranked = rank(&choice, &devices);
        assert_eq!(ranked[0].name, "AirPods Pro");
        assert_eq!(ranked.len(), 3);
    }

    #[test]
    fn a_wired_default_comes_first_and_the_tap_never_appears() {
        let devices = vec![
            device("Deino System Audio", Transport::Virtual, false),
            device("USB Mic", Transport::Usb, true),
            device("MacBook Pro Microphone", Transport::BuiltIn, false),
        ];
        let ranked = rank(&MicChoice::default(), &devices);
        assert_eq!(names(&ranked), ["USB Mic", "MacBook Pro Microphone"]);
    }

    #[test]
    fn a_missing_picked_device_falls_back_to_the_default() {
        let choice = MicChoice {
            preferred: Some("Gone".into()),
        };
        let devices = vec![device("MacBook Pro Microphone", Transport::BuiltIn, true)];
        assert_eq!(names(&rank(&choice, &devices)), ["MacBook Pro Microphone"]);
    }

    #[test]
    fn hands_free_means_bluetooth_at_a_call_rate() {
        assert!(is_hands_free(Transport::Bluetooth, 16_000));
        assert!(!is_hands_free(Transport::Bluetooth, 48_000));
        assert!(!is_hands_free(Transport::BuiltIn, 16_000));
    }
}
