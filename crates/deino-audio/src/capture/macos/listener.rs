//! Core Audio property listeners that own their context.
//!
//! A [`Listener`] registers one C callback for a set of properties of an
//! audio object and owns the callback's context until it unregisters. If
//! unregistering fails, the context is deliberately leaked rather than freed
//! under a callback that may still run (anarlog's device-monitor guard,
//! MIT). Callbacks run on a Core Audio notification thread: they only flip
//! an atomic or `try_send` an enum.
//!
//! [`CoreAudioWatch`] is the [`SystemWatch`] over the system object's
//! default-input, default-output and device-list properties.

use std::sync::mpsc::SyncSender;

use cidre::core_audio as ca;
use cidre::os;

use crate::capture::events::{DeviceEvent, Subscription, SystemEvent, SystemWatch};

/// A registered listener and its owned context.
pub struct Listener<T: 'static> {
    object: ca::Obj,
    addresses: Vec<ca::PropAddr>,
    callback: ca::PropListenerFn<T>,
    context: *mut T,
}

impl<T: 'static> std::fmt::Debug for Listener<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Listener")
            .field("object", &self.object.0)
            .field("properties", &self.addresses.len())
            .finish_non_exhaustive()
    }
}

// SAFETY: the context is only touched by the callback (through `&T`) and by
// `drop`, after every registration is gone; `T: Send + Sync` makes both fine
// from any thread.
#[allow(unsafe_code, reason = "a raw context pointer shared with a C callback")]
unsafe impl<T: Send + Sync + 'static> Send for Listener<T> {}

impl<T: Send + Sync + 'static> Listener<T> {
    /// Registers `callback` for each of `addresses` on `object`. On failure
    /// the registrations made so far are rolled back.
    pub fn register(
        object: ca::Obj,
        addresses: Vec<ca::PropAddr>,
        context: T,
        callback: ca::PropListenerFn<T>,
    ) -> os::Result<Self> {
        let mut listener = Self {
            object,
            addresses: Vec::with_capacity(addresses.len()),
            callback,
            context: Box::into_raw(Box::new(context)),
        };
        for address in addresses {
            object.add_prop_listener(&address, callback, listener.context)?;
            listener.addresses.push(address);
        }
        Ok(listener)
    }
}

impl<T: 'static> Drop for Listener<T> {
    #[allow(
        unsafe_code,
        reason = "frees the context Box::into_raw made, once no callback can run"
    )]
    fn drop(&mut self) {
        let mut clean = true;
        for address in &self.addresses {
            clean &= self
                .object
                .remove_prop_listener(address, self.callback, self.context)
                .is_ok();
        }
        if clean {
            // SAFETY: every registration is removed, so no callback holds the
            // pointer, and it came from `Box::into_raw` in `register`.
            drop(unsafe { Box::from_raw(self.context) });
        } else {
            log::error!(target: "deino_audio::capture", "a Core Audio listener did not unregister; leaking its context");
        }
    }
}

/// `kAudioObjectSystemObject`, the object holding the default devices.
const SYSTEM_OBJECT: ca::Obj = ca::Obj(1);

/// The context of the system-wide device listener.
struct DeviceContext {
    events: SyncSender<SystemEvent>,
}

/// Maps changed system properties to events.
#[allow(
    unsafe_code,
    reason = "a Core Audio listener callback reading its context and addresses"
)]
extern "C-unwind" fn on_system_change(
    _object: ca::Obj,
    count: u32,
    addresses: *const ca::PropAddr,
    context: *mut DeviceContext,
) -> os::Status {
    if context.is_null() || addresses.is_null() {
        return os::Status::NO_ERR;
    }
    // SAFETY: Core Audio passes `count` valid addresses and the context
    // registered with this callback, alive until the listener is removed.
    let (context, changed) = unsafe {
        let len = usize::try_from(count).unwrap_or_default();
        (&*context, std::slice::from_raw_parts(addresses, len))
    };
    for address in changed {
        if let Some(event) = event_for(address.selector) {
            context.events.try_send(SystemEvent::Device(event)).ok();
        }
    }
    os::Status::NO_ERR
}

/// The event a changed system property means.
fn event_for(selector: ca::PropSelector) -> Option<DeviceEvent> {
    match selector.0 {
        s if s == ca::PropSelector::HW_DEFAULT_INPUT_DEVICE.0 => {
            Some(DeviceEvent::DefaultInputChanged)
        }
        s if s == ca::PropSelector::HW_DEFAULT_OUTPUT_DEVICE.0 => {
            Some(DeviceEvent::DefaultOutputChanged)
        }
        s if s == ca::PropSelector::HW_DEVICES.0 => Some(DeviceEvent::DeviceListChanged),
        _ => None,
    }
}

/// Watches the default devices and the device list.
#[derive(Clone, Copy, Debug, Default)]
pub struct CoreAudioWatch;

impl Subscription for Listener<DeviceContext> {}

impl SystemWatch for CoreAudioWatch {
    fn watch(&self, events: SyncSender<SystemEvent>) -> Box<dyn Subscription> {
        let addresses = vec![
            ca::PropSelector::HW_DEFAULT_INPUT_DEVICE.global_addr(),
            ca::PropSelector::HW_DEFAULT_OUTPUT_DEVICE.global_addr(),
            ca::PropSelector::HW_DEVICES.global_addr(),
        ];
        let context = DeviceContext { events };
        match Listener::register(SYSTEM_OBJECT, addresses, context, on_system_change) {
            Ok(listener) => Box::new(listener),
            Err(status) => {
                log::warn!(target: "deino_audio::capture", "device changes are not watched (OSStatus {status:?})");
                Box::new(crate::capture::events::NoSubscription)
            }
        }
    }
}
