//! Whether a recording captures the screen, and what: [`decide`].
//!
//! Screen capture is off by default. The user can turn it on "for detected
//! meetings" (Circleback's rule): a recording captures when meeting
//! detection sees a meeting app at its start. A meeting the user turned
//! off stays off, Screen Recording must already be granted (Deino never
//! makes macOS ask from a background start), and at least one of the video
//! or the snapshots must be wanted. Pure: the app gathers the inputs.

use std::path::PathBuf;

use deino_domain::{BundleId, MeetingId};

use crate::frame::FrameRate;
use crate::sampler::SnapshotPolicy;

/// The user's screen capture settings, as this crate reads them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CaptureSettings {
    /// Capture when a detected meeting is recorded. Off by default.
    pub detected_meetings: bool,
    /// Record the meeting window as `video.mp4`.
    pub video: bool,
    /// The video's frame rate.
    pub frame_rate: FrameRate,
    /// Keep snapshots of the shared screen.
    pub snapshots: bool,
}

/// What [`decide`] looks at when a recording starts.
#[derive(Clone, Debug)]
pub struct PlanInput {
    /// The recording's meeting.
    pub meeting: MeetingId,
    /// The meeting's recording folder, where the files go.
    pub folder: PathBuf,
    /// The settings.
    pub settings: CaptureSettings,
    /// The meeting app detection sees, if any.
    pub meeting_app: Option<BundleId>,
    /// The user turned capture off for this meeting.
    pub turned_off: bool,
    /// Screen Recording is granted (read without prompting).
    pub permitted: bool,
}

/// One recording's capture: whose window, into which folder, what to keep.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CapturePlan {
    /// The recording's meeting.
    pub meeting: MeetingId,
    /// The meeting's recording folder.
    pub folder: PathBuf,
    /// The app whose window is captured.
    pub app: BundleId,
    /// The video's frame rate, or `None` for no video.
    pub video: Option<FrameRate>,
    /// The snapshot policy, or `None` for no snapshots.
    pub snapshots: Option<SnapshotPolicy>,
}

/// Why a recording does not capture the screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Skip {
    /// Screen capture is off in Settings.
    Off,
    /// The user turned it off for this meeting.
    TurnedOff,
    /// Neither the video nor the snapshots are wanted.
    NothingWanted,
    /// No meeting app was detected, so there is no meeting window.
    NoMeetingApp,
    /// Screen Recording is not granted.
    NoPermission,
}

/// The plan for a recording starting with `input`, or why there is none.
pub fn decide(input: PlanInput) -> Result<CapturePlan, Skip> {
    let settings = input.settings;
    if !settings.detected_meetings {
        return Err(Skip::Off);
    }
    if input.turned_off {
        return Err(Skip::TurnedOff);
    }
    if !settings.video && !settings.snapshots {
        return Err(Skip::NothingWanted);
    }
    let app = input.meeting_app.ok_or(Skip::NoMeetingApp)?;
    if !input.permitted {
        return Err(Skip::NoPermission);
    }
    Ok(CapturePlan {
        meeting: input.meeting,
        folder: input.folder,
        app,
        video: settings.video.then_some(settings.frame_rate),
        snapshots: settings.snapshots.then(SnapshotPolicy::default),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input() -> PlanInput {
        PlanInput {
            meeting: MeetingId::new("meeting"),
            folder: PathBuf::from("/recordings/meeting"),
            settings: CaptureSettings {
                detected_meetings: true,
                video: true,
                frame_rate: FrameRate::DEFAULT,
                snapshots: true,
            },
            meeting_app: BundleId::try_from("us.zoom.xos").ok(),
            turned_off: false,
            permitted: true,
        }
    }

    #[test]
    fn a_detected_meeting_captures_video_and_snapshots_when_turned_on() {
        let plan = decide(input()).unwrap();

        assert_eq!(plan.app.as_str(), "us.zoom.xos");
        assert_eq!(plan.video, Some(FrameRate::DEFAULT));
        assert_eq!(plan.snapshots, Some(SnapshotPolicy::default()));
    }

    #[test]
    fn capture_is_off_by_default_and_per_meeting() {
        let mut off = input();
        off.settings.detected_meetings = false;
        assert_eq!(decide(off), Err(Skip::Off));

        let mut turned_off = input();
        turned_off.turned_off = true;
        assert_eq!(decide(turned_off), Err(Skip::TurnedOff));
    }

    #[test]
    fn it_needs_a_meeting_app_the_permission_and_something_to_keep() {
        let mut manual = input();
        manual.meeting_app = None;
        assert_eq!(decide(manual), Err(Skip::NoMeetingApp));

        let mut refused = input();
        refused.permitted = false;
        assert_eq!(decide(refused), Err(Skip::NoPermission));

        let mut nothing = input();
        nothing.settings.video = false;
        nothing.settings.snapshots = false;
        assert_eq!(decide(nothing), Err(Skip::NothingWanted));
    }

    #[test]
    fn snapshots_alone_record_no_video() {
        let mut stills = input();
        stills.settings.video = false;
        let plan = decide(stills).unwrap();

        assert_eq!(plan.video, None);
        assert!(plan.snapshots.is_some());
    }
}
