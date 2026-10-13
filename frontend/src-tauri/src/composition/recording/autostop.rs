//! Wiring for auto-stop and meeting boundaries (CAP-07): the service over
//! the recorder, meeting detection (who holds the microphone), the synced
//! calendar and the settings; the ticker thread; and the Mac's sleep
//! notification, which stops a recording before the lid closes.

use std::sync::{Arc, Mutex};

use deino_audio::capture::events::Subscription;
use deino_core::settings::SettingsService;
use deino_detect::DetectionService;
use deino_detect::autostop::{AutoStopService, AutoStopTicker, service::AutoStopPorts};
use deino_domain::Clock;
use tauri::{AppHandle, Manager};

use crate::features::recording::autostop::{
    RecorderProbe, SettingsAutoStop, SyncedWindows, TauriAutoStopActions,
};

/// Keeps auto-stop's thread and sleep listener for the app's lifetime.
pub(crate) struct AutoStopRuntime {
    _ticker: Arc<AutoStopTicker>,
    _power: Mutex<Option<Box<dyn Subscription>>>,
}

/// Manages `Arc<AutoStopService>` and starts its ticker. Needs the settings
/// and meeting detection; the recorder and calendar are found when attached.
pub(crate) fn attach_autostop(app: &AppHandle, clock: &Arc<dyn Clock>) {
    let (Some(settings), Some(detection)) = (
        app.try_state::<Arc<SettingsService>>()
            .map(|s| Arc::clone(&s)),
        app.try_state::<Arc<DetectionService>>()
            .map(|d| Arc::clone(&d)),
    ) else {
        return log::error!(target: "recording", "settings or detection are missing; auto-stop is off");
    };
    let service = Arc::new(AutoStopService::new(
        AutoStopPorts {
            recording: Arc::new(RecorderProbe(app.clone())),
            microphone: detection,
            calendar: Arc::new(SyncedWindows::new(app.clone(), Arc::clone(&settings))),
            settings: Arc::new(SettingsAutoStop(settings)),
            actions: Arc::new(TauriAutoStopActions::new(app.clone(), ending_card)),
        },
        Arc::clone(clock),
    ));
    app.manage(Arc::clone(&service));
    let ticker = match AutoStopTicker::spawn(service) {
        Ok(ticker) => Arc::new(ticker),
        Err(error) => return log::error!(target: "recording", "auto-stop did not start: {error}"),
    };
    let power = watch_sleep(Arc::clone(&ticker));
    app.manage(AutoStopRuntime {
        _ticker: ticker,
        _power: Mutex::new(power),
    });
}

/// The "Ending…" card shares the meeting prompt's top-right window, whose
/// page shows it while no prompt is up.
fn ending_card(app: &AppHandle, showing: bool) {
    if showing {
        crate::features::detection::prompt::show(app);
    } else {
        crate::features::detection::prompt::hide(app);
    }
}

/// Tells the ticker when the Mac is about to sleep.
#[cfg(target_os = "macos")]
fn watch_sleep(ticker: Arc<AutoStopTicker>) -> Option<Box<dyn Subscription>> {
    use deino_audio::capture::SystemWatch;
    use deino_audio::capture::events::{PowerEvent, SystemEvent};

    let (events, received) = std::sync::mpsc::sync_channel::<SystemEvent>(8);
    let subscription = deino_audio::capture::macos::power::PowerWatch.watch(events);
    let forwarded = std::thread::Builder::new()
        .name("deino-autostop-power".into())
        .spawn(move || {
            let sleeping = SystemEvent::Power(PowerEvent::WillSleep);
            for _ in received.iter().filter(|event| *event == sleeping) {
                ticker.will_sleep();
            }
        });
    match forwarded {
        Ok(_) => Some(subscription),
        Err(error) => {
            log::warn!(target: "recording", "sleep is not watched for auto-stop: {error}");
            None
        }
    }
}

/// Other platforms: auto-stop's tick gap stands in for the sleep notice.
#[cfg(not(target_os = "macos"))]
fn watch_sleep(_ticker: Arc<AutoStopTicker>) -> Option<Box<dyn Subscription>> {
    None
}
