//! [`SignalSnapshot`]: everything the detector knows at one moment, and
//! [`Candidate`]: the meeting it concludes is happening.

use deino_domain::BundleId;

use crate::calendar::LinkedEvent;
use crate::model::{AppIdentity, MicClient, Platform};
use crate::url::MeetingUrl;

/// The evidence at one moment, already attributed to apps.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SignalSnapshot {
    /// Apps capturing from the microphone (never Deino itself).
    pub clients: Vec<MicClient>,
    /// The default input device is running for some process.
    pub microphone_running: bool,
    /// A camera is running for some process.
    pub camera_running: bool,
    /// The frontmost app.
    pub frontmost: Option<AppIdentity>,
    /// Browser windows showing a meeting room (needs Accessibility).
    pub browser_meetings: Vec<BrowserMeeting>,
    /// The calendar event a call now would belong to.
    pub event: Option<LinkedEvent>,
}

/// A browser window showing a meeting room.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrowserMeeting {
    /// The browser.
    pub browser: AppIdentity,
    /// The room.
    pub url: MeetingUrl,
    /// The browser is the frontmost app.
    pub focused: bool,
}

/// How long a candidate must hold steady before the prompt shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dwell {
    /// Strong evidence (a meeting app, a meeting URL, camera with microphone,
    /// a calendar event): prompt within about three seconds.
    Short,
    /// Weak evidence (a browser or another app holding the microphone):
    /// prompt only after a sustained hold, as most brief holds are not calls.
    Sustained,
}

/// Identifies one meeting across evaluations: the app (`app:us.zoom.xos`)
/// or the browser room (`url:meet.google.com/abc-defg-hij`). Never logged.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CandidateKey(String);

impl CandidateKey {
    /// The key of a meeting in `app`.
    pub fn app(bundle_id: &BundleId) -> Self {
        Self(format!("app:{bundle_id}"))
    }

    /// The key of a meeting in a browser room.
    pub fn room(url: &MeetingUrl) -> Self {
        Self(format!("url:{}", url.room))
    }
}

/// The meeting the evidence points to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Candidate {
    /// Which meeting this is.
    pub key: CandidateKey,
    /// The app it happens in, named in the prompt; its cooldown and ignore
    /// entry key on it.
    pub app: AppIdentity,
    /// The service.
    pub platform: Platform,
    /// How long it must hold before the prompt.
    pub dwell: Dwell,
    /// The calendar event it belongs to.
    pub event: Option<LinkedEvent>,
}
