//! What can be done to a running recording, and what its watchers read:
//! pause and resume, the microphone switch and the system-audio toggle
//! (CAP-10), the panel's levels and the lanes' activity (CAP-07's
//! auto-stop).

use deino_audio::capture::MicChoice;

use super::RecordingService;
use crate::activity::LaneReading;
use crate::error::RecordingError;
use crate::events::{AudioLevels, RecordingPhase, RecordingSnapshot};
use crate::lane::PerLane;

impl RecordingService {
    /// Pauses capture; the recording's timeline skips the pause.
    pub fn pause(&self) -> Result<RecordingSnapshot, RecordingError> {
        self.in_phase(RecordingPhase::Recording)?;
        self.with_session(super::Session::pause)?;
        let now = self.deps.recorder.wall_clock.now();
        Ok(self.state.update(|state| {
            state.phase = RecordingPhase::Paused;
            state.paused_at = Some(now);
        }))
    }

    /// Resumes a paused recording.
    pub fn resume(&self) -> Result<RecordingSnapshot, RecordingError> {
        self.in_phase(RecordingPhase::Paused)?;
        self.with_session(super::Session::resume)?;
        let now = self.deps.recorder.wall_clock.now();
        Ok(self.state.update(|state| {
            let paused = state
                .paused_at
                .take()
                .map(|at| u64::try_from((now - at).num_milliseconds()).unwrap_or_default())
                .unwrap_or_default();
            state.paused_ms += paused;
            state.phase = RecordingPhase::Recording;
        }))
    }

    /// Records from the microphone named `name` from now on (`None`: the
    /// system default), without a gap in the meeting.
    pub fn set_microphone(
        &self,
        name: Option<String>,
    ) -> Result<RecordingSnapshot, RecordingError> {
        let choice = MicChoice {
            preferred: name.clone(),
        };
        self.with_session(|session| session.set_microphone(choice))?;
        Ok(self.state.update(|state| state.microphone = name))
    }

    /// Records system audio (the other side of the call), or stops and
    /// records silence on its track.
    pub fn set_system_audio(&self, enabled: bool) -> Result<RecordingSnapshot, RecordingError> {
        self.with_session(|session| session.set_system_audio(enabled))?;
        Ok(self.state.update(|state| state.system_audio = enabled))
    }

    /// Each track's loudest sample since the previous call, for one meter.
    pub fn levels(&self) -> Option<AudioLevels> {
        self.with_session(super::Session::levels).ok()
    }

    /// What each lane has heard so far, while recording.
    pub fn activity(&self) -> Option<PerLane<LaneReading>> {
        self.with_session(super::Session::activity).ok()
    }

    /// Fails unless the recorder is in `phase`.
    fn in_phase(&self, phase: RecordingPhase) -> Result<(), RecordingError> {
        if self.state.get().phase == phase {
            Ok(())
        } else {
            Err(RecordingError::NotRecording)
        }
    }
}
