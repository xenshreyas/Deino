//! Which window to capture, and at what size.
//!
//! [`pick_window`] chooses the meeting app's window from the windows macOS
//! shares: an ordinary (layer 0), on-screen window of the app's bundle, at
//! least [`MIN_SIDE`] points on each side (toolbars, the call's mini
//! window and menus are smaller), the largest first. [`stream_size`] is
//! the pixel size frames are captured at: the window's backing size,
//! scaled down to fit [`MAX_SIDE`] and rounded to even numbers (H.264
//! works on 2 × 2 chroma blocks).

use deino_domain::BundleId;

/// The smallest window side (points) that can be a meeting window.
pub const MIN_SIDE: f64 = 200.0;
/// The longest captured side in pixels: enough for slide text, small enough
/// for a 1 fps H.264 stream of about 110 MB an hour.
pub const MAX_SIDE: u32 = 1920;

/// One window macOS can share, as plain data.
#[derive(Clone, Debug, PartialEq)]
pub struct WindowInfo {
    /// The window number (`CGWindowID`).
    pub id: u32,
    /// The app that owns it.
    pub bundle_id: Option<BundleId>,
    /// Its size in points.
    pub width: f64,
    /// Its size in points.
    pub height: f64,
    /// The window layer: 0 for ordinary windows.
    pub layer: i64,
    /// Whether it is on screen now (not minimised, its app not hidden).
    pub on_screen: bool,
    /// Its title, for the picker only (never logged).
    pub title: Option<String>,
    /// Its app's name, for the picker.
    pub app_name: Option<String>,
}

impl WindowInfo {
    /// Whether this can be `app`'s meeting window.
    fn is_candidate(&self, app: &BundleId) -> bool {
        self.bundle_id.as_ref() == Some(app)
            && self.layer == 0
            && self.on_screen
            && self.width >= MIN_SIDE
            && self.height >= MIN_SIDE
    }

    fn area(&self) -> f64 {
        self.width * self.height
    }
}

/// `app`'s meeting window among `windows`: its largest ordinary on-screen
/// window, if any.
pub fn pick_window<'a>(windows: &'a [WindowInfo], app: &BundleId) -> Option<&'a WindowInfo> {
    windows
        .iter()
        .filter(|window| window.is_candidate(app))
        .max_by(|a, b| a.area().total_cmp(&b.area()))
}

/// Whether window `id` is on screen among `windows`: false when it is
/// minimised, its app hidden, or the window gone.
pub fn is_visible(windows: &[WindowInfo], id: u32) -> bool {
    windows
        .iter()
        .any(|window| window.id == id && window.on_screen)
}

/// The capture size in pixels of a `width × height` point window on a
/// display with `scale` pixels per point (see the module docs).
pub fn stream_size(width: f64, height: f64, scale: f64) -> (u32, u32) {
    let (pixels_wide, pixels_high) = (width * scale.max(1.0), height * scale.max(1.0));
    let longest = pixels_wide.max(pixels_high).max(1.0);
    let fit = (f64::from(MAX_SIDE) / longest).min(1.0);
    (even(pixels_wide * fit), even(pixels_high * fit))
}

/// `value` rounded down to an even pixel count, at least 2.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the value is clamped to 2..=MAX_SIDE before the cast"
)]
fn even(value: f64) -> u32 {
    let clamped = value.clamp(2.0, f64::from(MAX_SIDE)) as u32;
    clamped & !1
}

#[cfg(test)]
mod tests {
    use super::*;

    fn zoom() -> BundleId {
        BundleId::try_from("us.zoom.xos").unwrap()
    }

    fn window(id: u32, size: (f64, f64), layer: i64, on_screen: bool) -> WindowInfo {
        WindowInfo {
            id,
            bundle_id: Some(zoom()),
            width: size.0,
            height: size.1,
            layer,
            on_screen,
            title: None,
            app_name: None,
        }
    }

    #[test]
    fn the_largest_ordinary_on_screen_window_of_the_app_is_picked() {
        let mut other = window(9, (2_000.0, 1_200.0), 0, true);
        other.bundle_id = BundleId::try_from("com.apple.Safari").ok();
        let windows = [
            window(1, (300.0, 250.0), 0, true),
            window(2, (1_400.0, 900.0), 0, true),
            window(3, (1_600.0, 1_000.0), 0, false),
            window(4, (1_800.0, 30.0), 25, true),
            window(5, (150.0, 900.0), 0, true),
            other,
        ];

        assert_eq!(pick_window(&windows, &zoom()).map(|w| w.id), Some(2));
        assert_eq!(pick_window(&windows[2..], &zoom()), None);
    }

    #[test]
    fn a_minimised_or_closed_window_is_not_visible() {
        let windows = [
            window(1, (800.0, 600.0), 0, true),
            window(2, (800.0, 600.0), 0, false),
        ];
        assert!(is_visible(&windows, 1));
        assert!(!is_visible(&windows, 2));
        assert!(!is_visible(&windows, 3));
    }

    #[test]
    fn frames_fit_the_longest_side_and_stay_even() {
        assert_eq!(stream_size(1_440.0, 900.0, 2.0), (1_920, 1_200));
        assert_eq!(stream_size(1_001.0, 701.0, 1.0), (1_000, 700));
        assert_eq!(stream_size(500.0, 2_000.0, 1.0), (480, 1_920));
        assert_eq!(stream_size(0.0, 0.0, 0.0), (2, 2));
    }
}
