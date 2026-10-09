//! Builders shared by this crate's unit tests.

use chrono::{DateTime, TimeZone, Utc};
use deino_domain::{BundleId, CalendarEventId};

use crate::calendar::LinkedEvent;
use crate::model::{AppIdentity, AudioProcess, MicClient, Pid};
use crate::ports::RunningApp;
use crate::snapshot::BrowserMeeting;
use crate::url::normalize;

/// Zoom's real bundle identifier, and the others the tests use.
pub(crate) const ZOOM: &str = "us.zoom.xos";
pub(crate) const SLACK: &str = "com.tinyspeck.slackmacgap";
pub(crate) const CHROME: &str = "com.google.Chrome";
pub(crate) const HANDY: &str = "com.pais.handy";
pub(crate) const PHOTO_BOOTH: &str = "com.apple.PhotoBooth";

/// 2026-10-09 10:00:00 UTC.
pub(crate) fn start() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, 9, 10, 0, 0).unwrap()
}

pub(crate) fn id(bundle: &str) -> BundleId {
    BundleId::try_from(bundle).unwrap()
}

/// The app with `bundle`, named after its last component.
pub(crate) fn app(bundle: &str) -> AppIdentity {
    let name = bundle.rsplit('.').next().unwrap_or(bundle).to_owned();
    AppIdentity {
        bundle_id: id(bundle),
        name,
    }
}

/// `bundle` capturing, optionally also playing.
pub(crate) fn client(bundle: &str, full_duplex: bool) -> MicClient {
    MicClient {
        app: app(bundle),
        pid: Pid(100),
        full_duplex,
    }
}

/// A Core Audio process.
pub(crate) fn process(pid: i32, bundle: Option<&str>, input: bool, output: bool) -> AudioProcess {
    AudioProcess {
        pid: Pid(pid),
        bundle_id: bundle.map(id),
        running_input: input,
        running_output: output,
    }
}

/// A running app.
pub(crate) fn running(bundle: &str, pid: i32, active: bool) -> RunningApp {
    RunningApp {
        app: app(bundle),
        pid: Pid(pid),
        active,
    }
}

/// A browser window on `url`.
pub(crate) fn room(browser: &str, url: &str, focused: bool) -> BrowserMeeting {
    BrowserMeeting {
        browser: app(browser),
        url: normalize(url).unwrap(),
        focused,
    }
}

/// A calendar event starting at `starts_at`.
pub(crate) fn event(starts_at: DateTime<Utc>) -> LinkedEvent {
    LinkedEvent {
        id: "01926b2c-0000-7000-8000-000000000001"
            .parse::<CalendarEventId>()
            .unwrap(),
        title: "Design review".to_owned(),
        starts_at,
    }
}
