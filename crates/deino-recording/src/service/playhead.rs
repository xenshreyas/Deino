//! [`Playhead`]: where the recorder is on a recording's own timeline, for
//! anything that stamps observations in recording time (CAP-05's speaker
//! tags) without touching the audio threads.

use deino_domain::{MeetingId, Millis};

use super::{RecordingService, Slot, lock};
use crate::events::RecordingPhase;

/// Where the recorder is now.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Playhead {
    /// `meeting` is recording, `at` from its start (pauses skipped), the
    /// same timeline its segments are on.
    Recording {
        /// The meeting being recorded.
        meeting: MeetingId,
        /// The offset on its timeline.
        at: Millis,
    },
    /// `meeting` is paused: its timeline stands still.
    Paused {
        /// The meeting being recorded.
        meeting: MeetingId,
    },
    /// Nothing is recording (or a recording is starting or stopping).
    Idle,
}

impl RecordingService {
    /// Where the recorder is now. Cheap: two locks and a clock read.
    pub fn playhead(&self) -> Playhead {
        let slot = lock(&self.slot);
        let Slot::Active(running) = &*slot else {
            return Playhead::Idle;
        };
        let session = &running.session;
        let meeting = session.meeting().clone();
        match self.state().phase {
            RecordingPhase::Recording => Playhead::Recording {
                at: session.position(self.deps.recorder.host_clock.as_ref()),
                meeting,
            },
            RecordingPhase::Paused => Playhead::Paused { meeting },
            RecordingPhase::Idle | RecordingPhase::Starting | RecordingPhase::Stopping => {
                Playhead::Idle
            }
        }
    }
}
