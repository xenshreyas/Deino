//! The two live lanes: what the user says ("Me") and what the call says
//! ("Them").
//!
//! A lane follows one recorded track from capture to transcript: the
//! microphone feeds Me, the system tap feeds Them. Each lane has its own
//! voice activity detector and its own transcription stream, and every
//! segment it produces is stored under the lane's [`Channel`].

use deino_audio::Track;
use deino_domain::Channel;

/// One of the two live lanes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Lane {
    /// The microphone: the user.
    Me,
    /// System audio: everyone else on the call.
    Them,
}

impl Lane {
    /// Both lanes, Me first.
    pub const ALL: [Self; 2] = [Self::Me, Self::Them];

    /// The recorded track the lane listens to.
    pub const fn track(self) -> Track {
        match self {
            Self::Me => Track::Microphone,
            Self::Them => Track::System,
        }
    }

    /// The transcript channel its segments are stored under.
    pub const fn channel(self) -> Channel {
        match self {
            Self::Me => Channel::Mic,
            Self::Them => Channel::System,
        }
    }

    /// The speech-to-text engine slot that streams this lane.
    pub const fn engine(self) -> deino_stt::Lane {
        match self {
            Self::Me => deino_stt::Lane::Microphone,
            Self::Them => deino_stt::Lane::System,
        }
    }

    /// The speaker label segments get until diarization names them.
    pub const fn speaker_label(self) -> &'static str {
        match self {
            Self::Me => "Me",
            Self::Them => "Them",
        }
    }

    /// The lane's name in logs and thread names.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Me => "me",
            Self::Them => "them",
        }
    }
}

/// One value per lane.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PerLane<T> {
    /// The Me lane's.
    pub me: T,
    /// The Them lane's.
    pub them: T,
}

impl<T> PerLane<T> {
    /// The value of `lane`.
    pub fn get(&self, lane: Lane) -> &T {
        match lane {
            Lane::Me => &self.me,
            Lane::Them => &self.them,
        }
    }

    /// The value of `lane`, mutably.
    pub fn get_mut(&mut self, lane: Lane) -> &mut T {
        match lane {
            Lane::Me => &mut self.me,
            Lane::Them => &mut self.them,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_microphone_is_me_and_system_audio_is_them() {
        assert_eq!(Lane::Me.track(), Track::Microphone);
        assert_eq!(Lane::Me.channel(), Channel::Mic);
        assert_eq!(Lane::Them.channel(), Channel::System);
        assert_eq!(Lane::Them.engine(), deino_stt::Lane::System);
        assert_eq!(Lane::Them.speaker_label(), "Them");
    }

    #[test]
    fn per_lane_values_are_addressed_by_lane() {
        let mut values = PerLane { me: 1, them: 2 };
        *values.get_mut(Lane::Them) += 1;
        assert_eq!((*values.get(Lane::Me), *values.get(Lane::Them)), (1, 3));
    }
}
