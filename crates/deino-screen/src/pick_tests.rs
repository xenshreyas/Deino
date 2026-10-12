//! The picker's entries and how a target is found again.

use super::*;

fn window(id: u32, app: (&str, &str), title: &str, size: (f64, f64)) -> WindowInfo {
    WindowInfo {
        id,
        bundle_id: BundleId::try_from(app.0).ok(),
        width: size.0,
        height: size.1,
        layer: 0,
        on_screen: true,
        title: Some(title.to_owned()),
        app_name: Some(app.1.to_owned()),
    }
}

fn display(id: u32) -> DisplayInfo {
    DisplayInfo {
        id,
        width: 1_512.0,
        height: 982.0,
        scale: 2.0,
    }
}

fn windows() -> Vec<WindowInfo> {
    let mut hidden = window(4, ("com.apple.Safari", "Safari"), "Docs", (1_200.0, 800.0));
    hidden.on_screen = false;
    vec![
        window(1, ("us.zoom.xos", "zoom.us"), "Zoom Meeting", (1_000.0, 700.0)),
        window(2, ("com.google.Chrome", "Google Chrome"), "Q3 deck", (1_400.0, 900.0)),
        window(3, (OWN_BUNDLE, "Deino"), "Deino", (1_200.0, 800.0)),
        hidden,
        window(5, ("com.apple.Notes", "Notes"), "", (900.0, 600.0)),
    ]
}

#[test]
fn screens_come_first_then_other_apps_windows_largest_first() {
    let choices = choices(&windows(), &[display(7), display(9)]);

    let labels: Vec<&str> = choices.iter().map(|choice| choice.label.as_str()).collect();
    assert_eq!(
        labels,
        [
            "Entire screen",
            "Screen 2",
            "Google Chrome — Q3 deck",
            "zoom.us — Zoom Meeting",
            "Notes"
        ]
    );
    assert_eq!(choices[0].source, Source::Display(7));
    assert_eq!(choices[2].source, Source::Window(2));
}

#[test]
fn a_target_is_found_again_or_reported_gone() {
    let windows = windows();
    let displays = [display(7)];
    let zoom = CaptureTarget::MeetingApp(BundleId::try_from("us.zoom.xos").unwrap());

    let app = resolve(&zoom, &windows, &displays).unwrap();
    assert_eq!((app.source, app.window), (Source::Window(1), Some(1)));
    let picked = resolve(&CaptureTarget::Picked(Source::Window(2)), &windows, &displays);
    assert_eq!(picked.map(|r| r.size), Some((1_400.0, 900.0)));
    let screen = resolve(&CaptureTarget::Picked(Source::Display(7)), &windows, &displays).unwrap();
    assert_eq!(screen.window, None, "a display is never paused as hidden");
    assert_eq!(resolve(&CaptureTarget::Picked(Source::Display(8)), &windows, &displays), None);
}

#[test]
fn a_source_crosses_ipc_as_kind_and_id() {
    let json = serde_json::to_value(Source::Display(7)).unwrap();
    assert_eq!(json, serde_json::json!({ "kind": "display", "id": 7 }));
}
