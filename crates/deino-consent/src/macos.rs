//! The macOS adapters behind [`crate::ports`]: the chat driver
//! (`AXUIElement`) and the keyboard's last key press (`CGEventSource`), next
//! to CAP-05's Accessibility ports for reading windows.

mod driver;

use std::sync::Arc;
use std::time::Duration;

use cidre::cg;

use crate::ports::{ConsentPlatform, KeyActivity};

/// This Mac's ports; `None` when speaker tags have no platform either.
pub(crate) fn platform() -> Option<ConsentPlatform> {
    Some(ConsentPlatform {
        tags: deino_speaker_tags::platform()?,
        driver: Arc::new(driver::AxDriver),
        keys: Arc::new(HidKeys),
    })
}

/// The last key press from the HID system state: physical keys only, so
/// nothing an app posts counts. Reading it needs no permission.
struct HidKeys;

impl KeyActivity for HidKeys {
    fn since_last_key(&self) -> Option<Duration> {
        let seconds =
            cg::EventSrcStateId::HidSystem.secs_since_last_event_type(cg::EventType::KEY_DOWN);
        Duration::try_from_secs_f64(seconds).ok()
    }
}
