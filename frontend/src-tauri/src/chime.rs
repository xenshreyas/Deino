//! The sound the meeting prompt and the meeting reminder make as they slide
//! in: a short two-note marimba (`sounds/marimba.wav`, rendered by
//! `scripts/brand/make-marimba.py`). It plays natively, through `NSSound`,
//! because the panels never activate and their pages may not play audio
//! without a click. It stays quiet when Settings → Notifications' sound
//! switch (`notifications.notificationSound`) is off.

use std::sync::Arc;

use deino_core::settings::SettingsService;
use tauri::{AppHandle, Manager, Wry};

/// The sound, built into the app.
const MARIMBA: &[u8] = include_bytes!("../sounds/marimba.wav");

/// Plays the chime, unless notification sounds are off. Returns at once.
pub(crate) fn play(app: &AppHandle<Wry>) {
    let Some(settings) = app.try_state::<Arc<SettingsService>>() else {
        return;
    };
    let settings = Arc::clone(&settings);
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        if !settings.get().await.notifications.notification_sound {
            return;
        }
        if let Err(error) = handle.run_on_main_thread(platform::play) {
            log::debug!(target: "chime", "could not reach the main thread: {error}");
        }
    });
}

#[cfg(target_os = "macos")]
mod platform {
    use std::cell::RefCell;

    use objc2::AllocAnyThread;
    use objc2::rc::Retained;
    use objc2_app_kit::NSSound;
    use objc2_foundation::NSData;

    thread_local! {
        /// The sound playing now, kept alive until the next one replaces it.
        static PLAYING: RefCell<Option<Retained<NSSound>>> = const { RefCell::new(None) };
    }

    /// Plays the marimba; on the main thread.
    pub(super) fn play() {
        let data = NSData::with_bytes(super::MARIMBA);
        let Some(sound) = NSSound::initWithData(NSSound::alloc(), &data) else {
            log::debug!(target: "chime", "could not read the chime");
            return;
        };
        if !sound.play() {
            log::debug!(target: "chime", "the chime did not play");
        }
        PLAYING.with(|playing| *playing.borrow_mut() = Some(sound));
    }
}

#[cfg(not(target_os = "macos"))]
mod platform {
    /// Elsewhere the panels have no chime yet.
    pub(super) fn play() {}
}
