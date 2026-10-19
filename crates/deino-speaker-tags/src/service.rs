//! [`SpeakerTags`]: what the app holds. It starts a [`Watcher`] for each
//! recording and answers Settings → Speaker tags.

use std::sync::{Arc, Mutex, PoisonError};

use deino_domain::MeetingId;

use crate::capture::WindowCapture;
use crate::ports::{Platform, RecordingClock, TagSink};
use crate::status::{AppTagStatus, LayoutHealth, statuses};
use crate::timeline::POLL_INTERVAL;
use crate::watch::{Watch, WatchParts};
use crate::watcher::Watcher;

/// What [`SpeakerTags`] is built over.
#[derive(Clone)]
pub struct SpeakerTagsParts {
    /// This Mac's Accessibility ports; `None` where there are none.
    pub platform: Option<Platform>,
    /// The recorder's clock.
    pub clock: Arc<dyn RecordingClock>,
    /// Where spans are stored.
    pub sink: Arc<dyn TagSink>,
    /// Where dumps of unreadable call windows go.
    pub capture: Arc<dyn WindowCapture>,
}

/// Speaker tags for every recording (see the module docs).
pub struct SpeakerTags {
    parts: SpeakerTagsParts,
    health: Arc<LayoutHealth>,
    current: Mutex<Option<Watcher>>,
}

impl SpeakerTags {
    /// Speaker tags over `parts`; nothing is watched yet.
    pub fn new(parts: SpeakerTagsParts) -> Self {
        Self {
            parts,
            health: Arc::default(),
            current: Mutex::new(None),
        }
    }

    /// Starts watching the recording of `meeting`. A previous watch is
    /// told to stop (it saves what it saw on its own thread). Without a
    /// platform, nothing happens: the meeting is named by voice.
    pub fn watch(&self, meeting: MeetingId) {
        let Some(platform) = self.parts.platform.clone() else {
            log::debug!(target: "deino_speaker_tags", "speaker tags are not supported here");
            return;
        };
        let watch = Watch::new(WatchParts {
            meeting,
            platform,
            clock: Arc::clone(&self.parts.clock),
            sink: Arc::clone(&self.parts.sink),
            health: Arc::clone(&self.health),
            capture: Arc::clone(&self.parts.capture),
        });
        let watcher = match Watcher::spawn(watch, POLL_INTERVAL) {
            Ok(watcher) => Some(watcher),
            Err(error) => {
                log::warn!(target: "deino_speaker_tags", "could not start the speaker-tag watcher: {error}");
                None
            }
        };
        let mut current = self.current.lock().unwrap_or_else(PoisonError::into_inner);
        // Dropping the old handle wakes its thread, which saves and ends.
        *current = watcher;
    }

    /// Each app's row for Settings → Speaker tags.
    pub fn statuses(&self) -> Vec<AppTagStatus> {
        let granted = self
            .parts
            .platform
            .as_ref()
            .is_some_and(|platform| platform.permission.granted());
        statuses(self.parts.platform.is_some(), granted, &self.health)
    }
}

#[cfg(test)]
mod tests {
    use deino_domain::CallApp;

    use super::*;
    use crate::capture::NoCapture;
    use crate::fake::{FakeMac, MemorySink, ScriptedClock};
    use crate::status::TagState;

    fn tags(platform: Option<Platform>) -> SpeakerTags {
        SpeakerTags::new(SpeakerTagsParts {
            platform,
            clock: ScriptedClock::new([]),
            sink: Arc::new(MemorySink::default()),
            capture: Arc::new(NoCapture),
        })
    }

    #[test]
    fn rows_follow_the_permission_and_the_platform() {
        let mac = FakeMac::granted();
        let with_mac = tags(Some(mac.platform()));
        let states = |tags: &SpeakerTags| -> Vec<TagState> {
            tags.statuses().into_iter().map(|row| row.state).collect()
        };

        assert_eq!(states(&with_mac), [TagState::Enabled; 3]);
        mac.set_granted(false);
        assert_eq!(states(&with_mac), [TagState::NeedsPermission; 3]);
        assert_eq!(states(&tags(None)), [TagState::Unsupported; 3]);
        assert_eq!(with_mac.statuses()[0].app, CallApp::Zoom);
    }

    #[test]
    fn watching_without_a_platform_does_nothing() {
        let tags = tags(None);

        tags.watch(MeetingId::new("meeting"));

        assert!(tags.current.lock().unwrap().is_none());
    }
}
