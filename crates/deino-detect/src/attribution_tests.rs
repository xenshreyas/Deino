use std::path::Path;

use super::*;
use crate::fake::FakeDirectory;
use crate::testing::{CHROME, app, process, running};

fn directory(running_apps: Vec<RunningApp>) -> FakeDirectory {
    let directory = FakeDirectory::default();
    directory.running.set(running_apps);
    directory
}

#[test]
fn helpers_resolve_to_the_outermost_app_bundle() {
    let helper = Path::new(
        "/Applications/Google Chrome.app/Contents/Frameworks/Google Chrome Framework.framework/Helpers/Google Chrome Helper (Renderer).app/Contents/MacOS/Google Chrome Helper (Renderer)",
    );
    let slack = Path::new(
        "/Applications/Slack.app/Contents/Frameworks/Slack Helper.app/Contents/MacOS/Slack Helper",
    );

    assert_eq!(
        outermost_app_bundle(helper),
        Some(Path::new("/Applications/Google Chrome.app"))
    );
    assert_eq!(
        outermost_app_bundle(slack),
        Some(Path::new("/Applications/Slack.app"))
    );
    assert_eq!(
        outermost_app_bundle(Path::new("/opt/homebrew/bin/ffmpeg")),
        None
    );
}

#[test]
fn a_process_outside_any_bundle_is_named_by_its_executable() {
    let ffmpeg = process_identity("ffmpeg", Pid(7)).unwrap();
    let spaced = process_identity("my recorder", Pid(7)).unwrap();
    let nameless = process_identity("", Pid(7)).unwrap();

    assert_eq!(
        (ffmpeg.bundle_id.as_str(), ffmpeg.name.as_str()),
        ("process.ffmpeg", "ffmpeg")
    );
    assert_eq!(spaced.bundle_id.as_str(), "process.my-recorder");
    assert_eq!(nameless.bundle_id.as_str(), "process.7");
}

#[test]
fn a_live_process_takes_the_app_the_directory_names() {
    let directory = directory(Vec::new());
    directory.pids.lock().insert(Pid(42), app("us.zoom.xos"));

    let attributed = attribute(&process(42, Some("us.zoom.xos"), true, true), &directory);

    assert_eq!(attributed, Some(app("us.zoom.xos")));
}

#[test]
fn an_exited_helper_maps_to_its_running_browser() {
    let directory = directory(vec![running(CHROME, 10, false)]);

    let attributed = attribute(
        &process(42, Some("com.google.Chrome.helper"), true, false),
        &directory,
    );

    assert_eq!(attributed, Some(app(CHROME)));
}

#[test]
fn an_exited_process_without_a_bundle_cannot_be_named() {
    assert_eq!(
        attribute(&process(42, None, true, false), &directory(Vec::new())),
        None
    );
}

#[test]
fn call_daemons_become_the_frontmost_call_app() {
    let daemon = process(42, Some("com.apple.avconferenced"), true, true);
    let facetime_front = directory(vec![
        running("com.apple.FaceTime", 1, true),
        running("com.apple.mobilephone", 2, false),
    ]);
    let phone_front = directory(vec![
        running("com.apple.FaceTime", 1, false),
        running("com.apple.mobilephone", 2, true),
    ]);

    let facetime = attribute(&daemon, &facetime_front).unwrap();
    let phone = attribute(&daemon, &phone_front).unwrap();

    assert_eq!(facetime.bundle_id.as_str(), "com.apple.FaceTime");
    assert_eq!(
        (phone.bundle_id.as_str(), phone.name.as_str()),
        ("com.apple.mobilephone", "iPhone Call")
    );
}

#[test]
fn call_daemons_use_the_only_running_call_app_and_stay_put_when_ambiguous() {
    let daemon = process(
        42,
        Some("com.apple.TelephonyUtilities.callservicesd"),
        true,
        true,
    );
    let only_facetime = directory(vec![running("com.apple.FaceTime", 1, false)]);
    let both_background = directory(vec![
        running("com.apple.FaceTime", 1, false),
        running("com.apple.mobilephone", 2, false),
    ]);

    assert_eq!(
        attribute(&daemon, &only_facetime)
            .unwrap()
            .bundle_id
            .as_str(),
        "com.apple.FaceTime"
    );
    assert_eq!(
        attribute(&daemon, &both_background)
            .unwrap()
            .bundle_id
            .as_str(),
        "com.apple.TelephonyUtilities.callservicesd"
    );
}

#[test]
fn the_webkit_audio_process_is_safari_while_safari_runs() {
    let webkit = process(42, Some("com.apple.WebKit.GPU"), true, false);

    let with_safari = attribute(
        &webkit,
        &directory(vec![running("com.apple.Safari", 1, false)]),
    );
    let without = attribute(&webkit, &directory(Vec::new()));

    assert_eq!(with_safari.unwrap().bundle_id.as_str(), "com.apple.Safari");
    assert_eq!(without.unwrap().bundle_id.as_str(), "com.apple.WebKit.GPU");
}
