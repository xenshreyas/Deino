//! The macOS capture adapters: the process tap, Core Audio listeners,
//! sleep/wake, device facts and the route probe.
//!
//! | Module | Role |
//! |---|---|
//! | [`tap`] | `SystemTap`: the global process tap (own PID excluded), stable aggregate UID + fallback |
//! | [`listener`] | Core Audio property listeners that own their context; `CoreAudioWatch` |
//! | [`power`] | `NSWorkspace` sleep/wake; `MacSystemWatch` (devices + power) |
//! | [`devices`] | Transports, headphone detection, `CoreAudioRoute` |

pub mod devices;
pub mod listener;
pub mod power;
pub mod tap;

pub use devices::CoreAudioRoute;
pub use power::MacSystemWatch;
pub use tap::SystemTap;

use cidre::{core_audio as ca, ns};

/// Whether this Mac can create a process tap: creates and destroys one
/// (muesli's probe). A denied *permission* can still create the tap and
/// deliver silence, which the user sees as an empty "Them" lane; the
/// onboarding explainer covers that case.
pub fn system_audio_available() -> bool {
    let desc = ca::TapDesc::with_mono_global_tap_excluding_processes(&ns::Array::new());
    desc.create_process_tap().is_ok()
}
