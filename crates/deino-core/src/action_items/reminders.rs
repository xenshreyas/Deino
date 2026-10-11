//! The morning reminder: at 09:00 local time, one system notification
//! listing the user's open items due today (PARITY_PLAN AI-05 "Reminders").
//!
//! [`DailyReminder::tick`] is called about once a minute by the app's
//! scheduler, which owns the timer; the rules live here, against the
//! [`Clock`] and the [`LocalZone`]. A Mac asleep at 09:00 still gets the
//! reminder when it wakes, until noon ([`CATCH_UP_UNTIL`]); after that the
//! day's reminder is skipped rather than arriving out of context. A day is
//! reminded at most once per run of the app.

use std::sync::{Arc, Mutex, PoisonError};

use async_trait::async_trait;
use chrono::{NaiveDate, NaiveTime};
use deino_domain::{Clock, LocalZone};

use super::{ActionItemError, ActionItemService, ActionItemView, TodayDigest};
use crate::settings::SettingsService;

/// When the reminder is due, local time.
pub const REMINDER_TIME: NaiveTime = match NaiveTime::from_hms_opt(9, 0, 0) {
    Some(time) => time,
    None => NaiveTime::MIN,
};
/// The latest local time a missed reminder is still shown.
pub const CATCH_UP_UNTIL: NaiveTime = match NaiveTime::from_hms_opt(12, 0, 0) {
    Some(time) => time,
    None => NaiveTime::MIN,
};
/// Item titles named in the notification's body; the rest are counted.
const TITLES_SHOWN: usize = 2;

/// A system notification's text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Notice {
    /// The headline.
    pub title: String,
    /// One line of detail.
    pub body: String,
}

/// Shows a system notification.
pub trait Notifier: Send + Sync {
    /// Shows `notice`; failures are the adapter's to log.
    fn notify(&self, notice: &Notice);
}

/// Whether the user wants the reminder now.
#[async_trait]
pub trait ReminderSwitch: Send + Sync {
    /// True when the reminder may be shown.
    async fn enabled(&self) -> bool;
}

#[async_trait]
impl ReminderSwitch for SettingsService {
    async fn enabled(&self) -> bool {
        self.get()
            .await
            .notifications
            .allows_action_item_reminders()
    }
}

/// What the reminder needs.
#[derive(Clone)]
pub struct ReminderDeps {
    /// The items.
    pub items: Arc<ActionItemService>,
    /// The user's switch.
    pub switch: Arc<dyn ReminderSwitch>,
    /// The system notifications.
    pub notifier: Arc<dyn Notifier>,
    /// The time.
    pub clock: Arc<dyn Clock>,
    /// The user's time zone.
    pub zone: Arc<dyn LocalZone>,
}

/// Sends the morning reminder once a day.
pub struct DailyReminder {
    deps: ReminderDeps,
    reminded: Mutex<Option<NaiveDate>>,
}

impl DailyReminder {
    /// A reminder that has not reminded yet.
    pub fn new(deps: ReminderDeps) -> Self {
        Self {
            deps,
            reminded: Mutex::new(None),
        }
    }

    /// Shows today's reminder if it is due and was not shown yet; returns
    /// what it showed.
    pub async fn tick(&self) -> Result<Option<Notice>, ActionItemError> {
        let local = self.deps.zone.local(self.deps.clock.now());
        let time = local.time();
        if time < REMINDER_TIME || time >= CATCH_UP_UNTIL || !self.first_today(local.date_naive()) {
            return Ok(None);
        }
        if !self.deps.switch.enabled().await {
            return Ok(None);
        }
        let notice = notice(&self.deps.items.today().await?);
        if let Some(notice) = &notice {
            self.deps.notifier.notify(notice);
        }
        Ok(notice)
    }

    /// Marks `today` as reminded; false if it already was.
    fn first_today(&self, today: NaiveDate) -> bool {
        let mut reminded = self.reminded.lock().unwrap_or_else(PoisonError::into_inner);
        if *reminded == Some(today) {
            return false;
        }
        *reminded = Some(today);
        true
    }
}

/// The notification for `digest`; `None` when nothing is due today.
pub fn notice(digest: &TodayDigest) -> Option<Notice> {
    let due = &digest.due_today;
    if due.is_empty() {
        return None;
    }
    let title = match due.len() {
        1 => "1 action item due today".to_owned(),
        count => format!("{count} action items due today"),
    };
    let body = match digest.overdue.len() {
        0 => names(due),
        overdue => format!("{} · {overdue} overdue", names(due)),
    };
    Some(Notice { title, body })
}

/// "Send the deck, Book the room and 3 more".
fn names(items: &[ActionItemView]) -> String {
    let shown: Vec<&str> = items
        .iter()
        .take(TITLES_SHOWN)
        .map(|item| item.title.as_str())
        .collect();
    match items.len().saturating_sub(TITLES_SHOWN) {
        0 => shown.join(", "),
        rest => format!("{} and {rest} more", shown.join(", ")),
    }
}

#[cfg(test)]
#[path = "reminders_tests.rs"]
mod tests;
