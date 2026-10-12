//! [`MacWindows`]: the windows `ScreenCaptureKit` can share, as plain
//! [`WindowInfo`]s, and the scale of the display a window is on.
//!
//! `SCShareableContent` answers through a completion handler on its own
//! queue; the handler copies what it needs into Rust values and sends
//! them back over a channel, so no Objective-C object crosses threads.
//! Only called once Screen Recording is granted (it would prompt otherwise).

use std::sync::mpsc::sync_channel;
use std::time::Duration;

use cidre::{ns, sc};
use deino_domain::BundleId;

use super::native::connect_window_server;
use crate::error::ScreenError;
use crate::ports::Windows;
use crate::pick::DisplayInfo;
use crate::target::WindowInfo;

/// How long `ScreenCaptureKit` may take to list the windows.
const LIST_TIMEOUT: Duration = Duration::from_secs(5);

/// The shareable windows through `SCShareableContent`.
#[derive(Clone, Copy, Debug)]
pub(crate) struct MacWindows;

impl Windows for MacWindows {
    fn list(&self) -> Result<Vec<WindowInfo>, ScreenError> {
        with_content(|content| content.windows().iter().map(info).collect())
    }

    fn scale(&self, id: u32) -> f64 {
        with_content(move |content| {
            let windows = content.windows();
            let window = windows.iter().find(|window| window.id() == id)?;
            let filter = sc::ContentFilter::with_desktop_independent_window(window);
            Some(f64::from(
                sc::ShareableContent::info_for_filter(&filter).point_pixel_scale(),
            ))
        })
        .ok()
        .flatten()
        .unwrap_or(2.0)
    }

    fn displays(&self) -> Result<Vec<DisplayInfo>, ScreenError> {
        with_content(|content| content.displays().iter().map(display_info).collect())
    }
}

/// Runs `read` over the current shareable content and returns its result.
pub(super) fn with_content<T: Send + 'static>(
    read: impl Fn(&sc::ShareableContent) -> T + 'static,
) -> Result<T, ScreenError> {
    connect_window_server();
    let (answer, answered) = sync_channel(1);
    sc::ShareableContent::current_with_ch(move |content, _error| {
        // The receiver may have timed out and gone; nothing to do then.
        answer.try_send(content.map(&read)).ok();
    });
    answered
        .recv_timeout(LIST_TIMEOUT)
        .map_err(|_| ScreenError::Timeout("SCShareableContent"))?
        .ok_or(ScreenError::Native {
            call: "SCShareableContent getShareableContent",
        })
}

/// A display as plain data, with its backing scale.
fn display_info(display: &sc::Display) -> DisplayInfo {
    let filter = sc::ContentFilter::with_display_excluding_windows(display, &ns::Array::new());
    let points = |side: isize| f64::from(i32::try_from(side).unwrap_or(0));
    DisplayInfo {
        id: display.display_id().0,
        width: points(display.width()),
        height: points(display.height()),
        scale: f64::from(sc::ShareableContent::info_for_filter(&filter).point_pixel_scale()),
    }
}

/// A window as plain data.
fn info(window: &sc::Window) -> WindowInfo {
    let frame = window.frame();
    let bundle_id = window
        .owning_app()
        .and_then(|app| BundleId::try_from(app.bundle_id().to_string()).ok());
    WindowInfo {
        id: window.id(),
        bundle_id,
        width: frame.size.width,
        height: frame.size.height,
        layer: i64::try_from(window.window_layer()).unwrap_or(i64::MAX),
        on_screen: window.is_on_screen(),
        title: window.title().map(|title| title.to_string()),
        app_name: window.owning_app().map(|app| app.app_name().to_string()),
    }
}
