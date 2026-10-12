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
use crate::pick::{CaptureTarget, Source};
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
    /// What is captured: the meeting app's window, or the user's pick.
    pub target: CaptureTarget,
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
        target: CaptureTarget::MeetingApp(app),
        video: settings.video.then_some(settings.frame_rate),
        snapshots: settings.snapshots.then(SnapshotPolicy::default),
    })
}

/// What [`picked`] looks at when the user picks a window or a screen in
/// the record panel.
#[derive(Clone, Debug)]
pub struct PickInput {
    /// The recording's meeting.
    pub meeting: MeetingId,
    /// The meeting's recording folder.
    pub folder: PathBuf,
    /// The settings.
    pub settings: CaptureSettings,
    /// What the user picked.
    pub source: Source,
    /// The meeting already has a video (an earlier capture of this
    /// recording): a second one would replace it, so the pick keeps
    /// snapshots only.
    pub has_video: bool,
    /// Screen Recording is granted (read without prompting).
    pub permitted: bool,
}

/// The plan for the user's pick, or why there is none. A pick is the
/// user's own choice, so it captures without a detected meeting app and
/// whether or not "Capture detected meetings" is on (a manual start).
pub fn picked(input: PickInput) -> Result<CapturePlan, Skip> {
    let settings = input.settings;
    let video = settings.video && !input.has_video;
    if !video && !settings.snapshots {
        return Err(Skip::NothingWanted);
    }
    if !input.permitted {
        return Err(Skip::NoPermission);
    }
    Ok(CapturePlan {
        meeting: input.meeting,
        folder: input.folder,
        target: CaptureTarget::Picked(input.source),
        video: video.then_some(settings.frame_rate),
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

        assert_eq!(
            plan.target,
            CaptureTarget::MeetingApp(BundleId::try_from("us.zoom.xos").unwrap())
        );
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

    fn pick(has_video: bool) -> PickInput {
        let mut settings = input().settings;
        settings.detected_meetings = false;
        PickInput {
            meeting: MeetingId::new("manual"),
            folder: PathBuf::from("/recordings/manual"),
            settings,
            source: Source::Display(1),
            has_video,
            permitted: true,
        }
    }

    #[test]
    fn a_pick_captures_a_manual_start_and_never_replaces_a_video() {
        let plan = picked(pick(false)).unwrap();
        assert_eq!(plan.target, CaptureTarget::Picked(Source::Display(1)));
        assert_eq!(plan.video, Some(FrameRate::DEFAULT));

        let again = picked(pick(true)).unwrap();
        assert_eq!(again.video, None, "the first video is kept");
        assert!(again.snapshots.is_some());

        let mut refused = pick(false);
        refused.permitted = false;
        assert_eq!(picked(refused), Err(Skip::NoPermission));
    }
}
