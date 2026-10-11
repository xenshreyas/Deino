//! What the system tells the capture supervisor, and the ports that tell it.
//!
//! Core Audio property listeners and the sleep/wake observer run on system
//! threads. They only `try_send` one small enum on a bounded channel and
//! return; every decision (debounce, restart, backoff) is made on the
//! supervisor's thread (the Zed device-listener pattern, reimplemented).

use std::sync::mpsc::SyncSender;

/// A change to the audio hardware.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeviceEvent {
    /// The system's default input device changed.
    DefaultInputChanged,
    /// The system's default output device changed (headphones plugged in,
    /// `AirPods` connected, ...).
    DefaultOutputChanged,
    /// A device appeared or disappeared.
    DeviceListChanged,
}

/// The Mac going to sleep or waking up.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PowerEvent {
    /// The system is about to sleep.
    WillSleep,
    /// The system woke up.
    DidWake,
}

/// Everything the supervisor's thread receives besides commands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SystemEvent {
    /// A hardware change.
    Device(DeviceEvent),
    /// A power change.
    Power(PowerEvent),
}

/// Keeps a subscription alive; dropping it unregisters the listeners.
pub trait Subscription: Send {}

/// A subscription that holds nothing (platforms without the listener).
#[derive(Debug)]
pub struct NoSubscription;

impl Subscription for NoSubscription {}

/// Starts listening for hardware or power changes.
pub trait SystemWatch: std::fmt::Debug + Send + Sync {
    /// Sends every change to `events` until the returned subscription drops.
    /// Senders never block: a full channel drops the event (the supervisor
    /// re-reads the hardware state on the next one anyway).
    fn watch(&self, events: SyncSender<SystemEvent>) -> Box<dyn Subscription>;
}

/// Watches nothing: tests, mock audio, and platforms without listeners.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoSystemWatch;

impl SystemWatch for NoSystemWatch {
    fn watch(&self, _events: SyncSender<SystemEvent>) -> Box<dyn Subscription> {
        Box::new(NoSubscription)
    }
}
