//! System-wide hotkeys (DEV-05): start or stop recording, a quick note and
//! Ask, working while Deino is in the background.
//!
//! - [`accelerator`]: [`Accelerator`], a validated combination such as
//!   `Ctrl+Alt+Cmd+R`, refusing plain ones and macOS's own;
//! - [`model`]: [`HotkeyAction`] (and the `crate::links::Intent` each runs),
//!   [`Hotkey`] and [`HotkeyState`] as Settings → Shortcuts shows them;
//! - [`ports`]: [`HotkeyRegistrar`], the system's hotkeys
//!   (`tauri-plugin-global-shortcut` in the app);
//! - [`service`]: [`HotkeyService`], which registers the saved hotkeys at
//!   startup and changes one at a time, conflicts refused;
//! - `fake`: [`FakeRegistrar`] behind `test-support`.
//!
//! The hotkeys are saved in `settings.shortcuts` ([`crate::settings::ShortcutSettings`]).

mod accelerator;
#[cfg(test)]
mod accelerator_tests;
mod error;
#[cfg(any(test, feature = "test-support"))]
mod fake;
mod model;
mod ports;
mod service;
#[cfg(test)]
mod service_tests;

pub use accelerator::Accelerator;
pub use error::HotkeyError;
#[cfg(any(test, feature = "test-support"))]
pub use fake::FakeRegistrar;
pub use model::{Hotkey, HotkeyAction, HotkeyState};
pub use ports::{HotkeyRegistrar, RegistrarRefused};
pub use service::HotkeyService;
