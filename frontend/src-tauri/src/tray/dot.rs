//! The dot on the menu bar icon while a recording is on: red and breathing
//! slowly while capturing, amber and still while paused (still too with
//! Reduce Motion). On macOS it is a small layer-backed view on the status
//! item's button, so the icon stays a template image the system tints;
//! elsewhere there is no dot yet.

use tauri::Runtime;
use tauri::tray::TrayIcon;

use super::view::Dot;

/// Shows `dot` on the icon, or takes it away. Returns at once; the change
/// happens on the main thread.
#[cfg(target_os = "macos")]
pub(super) fn show<R: Runtime>(tray: &TrayIcon<R>, dot: Option<Dot>) {
    let shown = tray.with_inner_tray_icon(move |inner| platform::show(inner.ns_status_item(), dot));
    if let Err(error) = shown {
        log::debug!(target: "tray", "could not update the recording dot: {error}");
    }
}

/// Elsewhere the icon has no dot yet.
#[cfg(not(target_os = "macos"))]
pub(super) fn show<R: Runtime>(_tray: &TrayIcon<R>, _dot: Option<Dot>) {}

#[cfg(target_os = "macos")]
mod platform {
    use std::cell::RefCell;

    use objc2::rc::Retained;
    use objc2::{MainThreadMarker, MainThreadOnly};
    use objc2_app_kit::{NSColor, NSStatusBarButton, NSStatusItem, NSView, NSWorkspace};
    use objc2_foundation::{NSNumber, NSPoint, NSRect, NSSize, NSString};
    use objc2_quartz_core::{
        CABasicAnimation, CAMediaTiming, CAMediaTimingFunction, kCAMediaTimingFunctionEaseInEaseOut,
    };

    use super::Dot;

    /// The dot's diameter, in points.
    const SIZE: f64 = 7.0;

    /// The icon's size (`tray-icon@2x.png`, 30 × 36 px), in points.
    const ICON_WIDTH: f64 = 15.0;
    const ICON_HEIGHT: f64 = 18.0;

    /// How far the dot's centre sits in from the icon's top-right corner.
    const NUDGE: f64 = 2.5;

    /// One breath, in seconds (it then breathes back out).
    const BREATH: f64 = 1.2;

    /// The opacity at the end of a breath.
    const FAINTEST: f32 = 0.45;

    thread_local! {
        /// The dot on screen and which one it is, so an unchanged dot keeps
        /// its breathing in step across updates.
        static SHOWN: RefCell<Option<(Dot, Retained<NSView>)>> = const { RefCell::new(None) };
    }

    /// Shows `dot`, or takes it away; on the main thread.
    pub(super) fn show(item: Option<Retained<NSStatusItem>>, dot: Option<Dot>) {
        let Some(mtm) = MainThreadMarker::new() else {
            return;
        };
        SHOWN.with(|shown| {
            let mut shown = shown.borrow_mut();
            if shown.as_ref().map(|(current, _)| *current) == dot {
                return;
            }
            if let Some((_, view)) = shown.take() {
                view.removeFromSuperview();
            }
            let button = item.and_then(|item| item.button(mtm));
            let (Some(dot), Some(button)) = (dot, button) else {
                return;
            };
            let view = make(&button, dot, mtm);
            button.addSubview(&view);
            *shown = Some((dot, view));
        });
    }

    /// The dot as a round, layer-backed view at the icon's top-right corner.
    fn make(button: &NSStatusBarButton, dot: Dot, mtm: MainThreadMarker) -> Retained<NSView> {
        let view = NSView::initWithFrame(NSView::alloc(mtm), frame(button));
        view.setWantsLayer(true);
        let Some(layer) = view.layer() else {
            return view;
        };
        layer.setCornerRadius(SIZE / 2.0);
        let colour = match dot {
            Dot::Recording => NSColor::systemRedColor(),
            Dot::Paused => NSColor::systemOrangeColor(),
        };
        layer.setBackgroundColor(Some(&colour.CGColor()));
        let still = NSWorkspace::sharedWorkspace().accessibilityDisplayShouldReduceMotion();
        if dot == Dot::Recording && !still {
            layer.addAnimation_forKey(&breathing(), Some(&NSString::from_str("breathe")));
        }
        view
    }

    /// Where the dot sits in the button, whichever way up the button draws.
    fn frame(button: &NSStatusBarButton) -> NSRect {
        let bounds = button.bounds();
        let icon_right = bounds.origin.x + f64::midpoint(bounds.size.width, ICON_WIDTH);
        let x = icon_right - NUDGE - SIZE / 2.0;
        let from_top = (bounds.size.height - ICON_HEIGHT) / 2.0 + NUDGE - SIZE / 2.0;
        let y = if button.isFlipped() {
            bounds.origin.y + from_top
        } else {
            bounds.origin.y + bounds.size.height - from_top - SIZE
        };
        NSRect::new(NSPoint::new(x, y), NSSize::new(SIZE, SIZE))
    }

    /// Opacity from full to faint and back, ease in and out, for ever.
    fn breathing() -> Retained<CABasicAnimation> {
        let animation =
            CABasicAnimation::animationWithKeyPath(Some(&NSString::from_str("opacity")));
        let full = NSNumber::new_f32(1.0);
        let faint = NSNumber::new_f32(FAINTEST);
        #[allow(unsafe_code, reason = "opacity animates between NSNumber floats")]
        // SAFETY: `opacity` is a float property, and both values are NSNumbers.
        unsafe {
            animation.setFromValue(Some(&full));
            animation.setToValue(Some(&faint));
        }
        animation.setDuration(BREATH);
        animation.setAutoreverses(true);
        animation.setRepeatCount(f32::INFINITY);
        #[allow(unsafe_code, reason = "reads a Core Animation constant")]
        // SAFETY: the constant is an immutable string Core Animation exports.
        let ease = unsafe { kCAMediaTimingFunctionEaseInEaseOut };
        animation.setTimingFunction(Some(&CAMediaTimingFunction::functionWithName(ease)));
        animation
    }
}
