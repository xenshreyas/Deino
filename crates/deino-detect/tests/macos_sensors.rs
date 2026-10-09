//! The real macOS sensors on this Mac. Ignored by default: they read the
//! hardware, so results depend on what is running. Run by hand:
//!
//! ```bash
//! cargo test -p deino-detect --test macos_sensors -- --ignored --nocapture
//! ```
//!
//! Start a call (or any app that uses the microphone) to see it attributed.

#![cfg(target_os = "macos")]

use std::sync::Arc;

use deino_detect::{NoCalendar, SignalReader, Waker, platform_sensors, reads_browser_urls};
use deino_domain::SystemClock;

#[test]
#[ignore = "reads this Mac's audio processes, cameras and apps"]
fn every_sensor_reads_and_every_listener_registers() {
    let sensors = platform_sensors().unwrap();
    let (waker, _signals) = Waker::channel();

    let mut watches = vec![
        sensors.processes.watch(&waker).unwrap(),
        sensors.microphone.watch(&waker).unwrap(),
        sensors.camera.watch(&waker).unwrap(),
    ];
    for watch in &mut watches {
        watch.refresh();
    }
    let processes = sensors.processes.read().unwrap();
    let microphone = sensors.microphone.read().unwrap();
    let camera = sensors.camera.read().unwrap();
    let snapshot = SignalReader::new(Arc::new(NoCalendar), Arc::new(SystemClock)).read(&sensors);

    println!(
        "{} audio processes; microphone running: {microphone}; camera running: {camera}",
        processes.len()
    );
    println!("Accessibility granted: {}", reads_browser_urls());
    println!("frontmost: {:?}", snapshot.frontmost.map(|app| app.name));
    for client in &snapshot.clients {
        println!(
            "capturing: {} ({}), full duplex: {}",
            client.app.name, client.app.bundle_id, client.full_duplex
        );
    }
    println!("{} browser meeting rooms", snapshot.browser_meetings.len());
    assert!(
        !processes.is_empty(),
        "Core Audio lists at least this test's own process"
    );
}
