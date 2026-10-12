//! [`ConsentReminders`]: the screen capture consent reminder shows the
//! first [`CONSENT_REMINDERS`] times a capture runs (Granola's consent
//! card), counted in `settings.screen.consentRemindersShown`.
//!
//! The record panel asks [`ConsentReminders::remind`] when a capture of
//! the recording starts. Asking again for the same meeting (the panel
//! reopened) answers the same without counting twice; "Don't show again"
//! ([`ConsentReminders::stop`]) uses the remaining reminders up.

use std::sync::{Arc, Mutex, PoisonError};

use async_trait::async_trait;
use deino_domain::MeetingId;

use crate::settings::{CONSENT_REMINDERS, SettingsError, SettingsService};

/// Where the count of reminders shown is kept.
#[async_trait]
pub trait ReminderCounter: Send + Sync {
    /// How many reminders have shown.
    async fn shown(&self) -> u8;

    /// Stores how many have shown.
    async fn set_shown(&self, shown: u8) -> Result<(), SettingsError>;
}

#[async_trait]
impl ReminderCounter for SettingsService {
    async fn shown(&self) -> u8 {
        self.get().await.screen.consent_reminders_shown
    }

    async fn set_shown(&self, shown: u8) -> Result<(), SettingsError> {
        self.update(|settings| settings.screen.consent_reminders_shown = shown)
            .await
            .map(|_| ())
    }
}

/// The reminder's rule (see the module docs).
pub struct ConsentReminders {
    counter: Arc<dyn ReminderCounter>,
    reminded: Mutex<Option<MeetingId>>,
}

impl ConsentReminders {
    /// The rule over the count.
    pub fn new(counter: Arc<dyn ReminderCounter>) -> Self {
        Self {
            counter,
            reminded: Mutex::new(None),
        }
    }

    /// Whether the capture of `meeting` shows the reminder, counting it
    /// the first time it is asked for that meeting.
    pub async fn remind(&self, meeting: &MeetingId) -> bool {
        if self.last().as_ref() == Some(meeting) {
            return true;
        }
        let shown = self.counter.shown().await;
        if shown >= CONSENT_REMINDERS {
            return false;
        }
        if let Err(error) = self.counter.set_shown(shown.saturating_add(1)).await {
            log::warn!(target: "screen", "could not count the consent reminder: {error}");
        }
        *self.reminded.lock().unwrap_or_else(PoisonError::into_inner) = Some(meeting.clone());
        true
    }

    /// "Don't show again": no capture shows the reminder any more.
    pub async fn stop(&self) -> Result<(), SettingsError> {
        *self.reminded.lock().unwrap_or_else(PoisonError::into_inner) = None;
        self.counter.set_shown(CONSENT_REMINDERS).await
    }

    fn last(&self) -> Option<MeetingId> {
        self.reminded
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU8, Ordering};

    use super::*;

    #[derive(Default)]
    struct Count(AtomicU8);

    #[async_trait]
    impl ReminderCounter for Count {
        async fn shown(&self) -> u8 {
            self.0.load(Ordering::SeqCst)
        }

        async fn set_shown(&self, shown: u8) -> Result<(), SettingsError> {
            self.0.store(shown, Ordering::SeqCst);
            Ok(())
        }
    }

    fn meeting(n: u8) -> MeetingId {
        MeetingId::new(format!("meeting-{n}"))
    }

    #[tokio::test]
    async fn the_first_three_captures_remind_once_each() {
        let count = Arc::new(Count::default());
        let reminders = ConsentReminders::new(count.clone());

        for n in 1..=3 {
            assert!(reminders.remind(&meeting(n)).await, "capture {n}");
            assert!(reminders.remind(&meeting(n)).await, "asked again");
        }
        assert_eq!(count.shown().await, 3);
        assert!(!reminders.remind(&meeting(4)).await);
        assert_eq!(count.shown().await, 3);
    }

    #[tokio::test]
    async fn dont_show_again_ends_the_reminders() {
        let count = Arc::new(Count::default());
        let reminders = ConsentReminders::new(count.clone());
        assert!(reminders.remind(&meeting(1)).await);

        reminders.stop().await.unwrap();

        assert!(!reminders.remind(&meeting(1)).await);
        assert!(!reminders.remind(&meeting(2)).await);
        assert_eq!(count.shown().await, CONSENT_REMINDERS);
    }
}
