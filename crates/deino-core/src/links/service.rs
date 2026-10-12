//! [`LinkService`]: follows links and hotkeys.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use super::error::LinkError;
use super::model::{DeepLink, Intent, Navigation};
use super::ports::{LockProbe, Recorder, Shell};

/// How many navigations wait for the window at most; older ones are dropped.
pub const MAX_PENDING: usize = 8;

/// Follows `deino://` links and hotkeys.
///
/// Recording runs at once, even while Deino is locked (recording is open
/// behind the lock, like the menu bar's). Anything that shows the library
/// waits in a queue the UI takes from with [`LinkService::take`], which
/// hands out nothing while Deino is locked, so a link never reveals a
/// meeting before the owner unlocks.
pub struct LinkService {
    recorder: Arc<dyn Recorder>,
    lock: Arc<dyn LockProbe>,
    shell: Arc<dyn Shell>,
    pending: Mutex<VecDeque<Navigation>>,
}

impl LinkService {
    /// A service over the recorder, the app lock and the window.
    pub fn new(
        recorder: Arc<dyn Recorder>,
        lock: Arc<dyn LockProbe>,
        shell: Arc<dyn Shell>,
    ) -> Self {
        Self {
            recorder,
            lock,
            shell,
            pending: Mutex::new(VecDeque::new()),
        }
    }

    /// Parses `text` and does what it says.
    pub async fn open(&self, text: &str) -> Result<(), LinkError> {
        let link = DeepLink::parse(text)?;
        log::info!(target: "links", "opening a {} link", link.route());
        self.perform(link.into()).await
    }

    /// Does what `intent` asks: records through the recorder, or queues a
    /// navigation and brings the window forward.
    pub async fn perform(&self, intent: Intent) -> Result<(), LinkError> {
        let recorded = match intent {
            Intent::StopRecording => self.recorder.stop().await,
            Intent::ToggleRecording if self.recorder.is_recording() => self.recorder.stop().await,
            Intent::StartRecording | Intent::ToggleRecording => self.recorder.start().await,
            Intent::Show(navigation) => {
                self.show(navigation);
                Ok(())
            }
        };
        recorded.map_err(LinkError::Recorder)
    }

    /// The navigations waiting, oldest first, and empties the queue. While
    /// Deino is locked it returns nothing and keeps them for the unlock.
    pub fn take(&self) -> Vec<Navigation> {
        if self.lock.is_locked() {
            return Vec::new();
        }
        self.queue().drain(..).collect()
    }

    fn show(&self, navigation: Navigation) {
        {
            let mut queue = self.queue();
            if queue.len() == MAX_PENDING {
                queue.pop_front();
            }
            queue.push_back(navigation);
        }
        if self.lock.is_locked() {
            log::info!(target: "links", "Deino is locked; the link waits for the unlock");
        }
        self.shell.focus();
        self.shell.navigation_waiting();
    }

    fn queue(&self) -> MutexGuard<'_, VecDeque<Navigation>> {
        // A panic while holding the lock leaves a whole queue behind.
        self.pending.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl std::fmt::Debug for LinkService {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LinkService")
            .field("pending", &self.queue().len())
            .finish_non_exhaustive()
    }
}
