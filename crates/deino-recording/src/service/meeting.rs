//! The meeting row a recording creates and closes.

use chrono::{DateTime, Utc};
use deino_audio::Frames;
use deino_core::meetings::{MeetingTiming, MeetingTitle, NewMeeting};
use deino_core::segments::Speaker;
use deino_domain::{
    Clock, MeetingEvent, MeetingId, MeetingSource, MeetingStatus, PaletteColor, Seconds, SpeakerId,
};

use crate::deps::RecordingDeps;
use crate::error::RecordingError;
use crate::lane::{Lane, PerLane};

/// The title of a recording started without one.
pub const DEFAULT_TITLE: &str = "Untitled meeting";

/// The Me speaker's palette colour.
const ME_COLOR: PaletteColor = PaletteColor::FIRST;

/// The Them speaker's palette colour.
const THEM_COLOR: PaletteColor = PaletteColor::SECOND;

/// A new meeting ID: `meeting-<UUIDv7>`, time-ordered.
pub fn new_id(clock: &dyn Clock) -> MeetingId {
    let id = deino_domain::SegmentId::generate(clock);
    MeetingId::new(format!("meeting-{id}"))
}

/// The requested title, or [`DEFAULT_TITLE`] when it is missing or blank.
pub fn title(requested: Option<&str>) -> Result<MeetingTitle, RecordingError> {
    let requested = requested.map(str::trim).filter(|title| !title.is_empty());
    Ok(MeetingTitle::parse(requested.unwrap_or(DEFAULT_TITLE))?)
}

/// The meeting row a recording creates.
pub struct MeetingRow<'a> {
    /// Its ID.
    pub meeting: &'a MeetingId,
    /// Its title.
    pub title: &'a MeetingTitle,
    /// When capture started.
    pub now: DateTime<Utc>,
    /// The Me and Them speaker IDs the transcript router already uses.
    pub speakers: PerLane<Option<SpeakerId>>,
}

/// Creates the meeting (`recording`, `live`, its recording folder) and its
/// Me and Them speakers.
pub async fn create(deps: &RecordingDeps, row: &MeetingRow<'_>) -> Result<(), RecordingError> {
    let folder = deino_audio::recording_dir(&deps.recordings, row.meeting);
    deps.meetings
        .create(&NewMeeting {
            id: row.meeting.clone(),
            title: row.title.clone(),
            status: MeetingStatus::Recording,
            source: MeetingSource::Live,
            created_at: row.now,
            started_at: Some(row.now),
            folder_path: Some(folder.to_string_lossy().into_owned()),
        })
        .await?;
    let me = deps
        .people
        .list()
        .await?
        .into_iter()
        .find(|person| person.is_me);
    for lane in Lane::ALL {
        let Some(id) = *row.speakers.get(lane) else {
            continue;
        };
        let speaker = speaker(row.meeting, lane, id, me.as_ref().map(|p| p.id));
        deps.speakers.add_speaker(&speaker).await?;
    }
    Ok(())
}

/// The speaker row of `lane`: Me is the user (linked to their person), Them
/// stands for everyone else until diarization names the voices.
fn speaker(
    meeting: &MeetingId,
    lane: Lane,
    id: SpeakerId,
    me: Option<deino_domain::PersonId>,
) -> Speaker {
    let (is_me, color) = match lane {
        Lane::Me => (true, ME_COLOR),
        Lane::Them => (false, THEM_COLOR),
    };
    Speaker {
        id,
        meeting_id: meeting.clone(),
        label: lane.speaker_label().to_owned(),
        person_id: if is_me { me } else { None },
        is_me,
        color,
    }
}

/// Records the meeting's timing and moves it to `processing` (which starts
/// the post-meeting pipeline).
pub async fn close(
    deps: &RecordingDeps,
    meeting: &MeetingId,
    (started_at, ended_at): (DateTime<Utc>, DateTime<Utc>),
    length: Frames,
) -> Result<(), RecordingError> {
    let timing = MeetingTiming {
        started_at: Some(started_at),
        ended_at: Some(ended_at),
        duration: Some(seconds(length)),
    };
    deps.meetings.set_timing(meeting, &timing, ended_at).await?;
    deps.lifecycle
        .transition(meeting, MeetingEvent::StopRecording)
        .await?;
    Ok(())
}

/// A recorded length in whole seconds.
pub fn seconds(length: Frames) -> Seconds {
    let secs = length.as_duration().as_secs();
    Seconds(u32::try_from(secs).unwrap_or(u32::MAX))
}
