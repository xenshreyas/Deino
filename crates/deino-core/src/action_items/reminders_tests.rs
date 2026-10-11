//! The morning reminder: when it fires, once a day, its text, and the switch.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use chrono::TimeDelta;

use super::*;
use crate::action_items::test_world::{Spec, World};

#[derive(Default)]
struct Shown(Mutex<Vec<Notice>>);

impl Notifier for Shown {
    fn notify(&self, notice: &Notice) {
        self.0.lock().unwrap().push(notice.clone());
    }
}

struct Switch(AtomicBool);

#[async_trait]
impl ReminderSwitch for Switch {
    async fn enabled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

struct Setup {
    world: World,
    reminder: DailyReminder,
    shown: Arc<Shown>,
    switch: Arc<Switch>,
}

/// Two items of mine due today, one overdue; "now" moved by `offset` from
/// Friday 09:00 local.
async fn setup(offset: TimeDelta) -> Setup {
    let world = World::new().await;
    for (title, due) in [
        ("Send the deck", "2026-10-09"),
        ("Book the room", "2026-10-09"),
        ("Old", "2026-10-07"),
    ] {
        world
            .item(Spec {
                title,
                assignee: Some(world.me),
                due: Some(due),
                ..Spec::default()
            })
            .await;
    }
    world.clock.advance(offset);
    let shown = Arc::new(Shown::default());
    let switch = Arc::new(Switch(AtomicBool::new(true)));
    let reminder = DailyReminder::new(ReminderDeps {
        items: Arc::new(world.service().0),
        switch: switch.clone(),
        notifier: shown.clone(),
        clock: world.clock.clone(),
        zone: world.zone.clone(),
    });
    Setup {
        world,
        reminder,
        shown,
        switch,
    }
}

#[tokio::test]
async fn at_nine_it_lists_todays_items_once() {
    let setup = setup(TimeDelta::zero()).await;

    let first = setup.reminder.tick().await.unwrap();
    setup.world.clock.advance(TimeDelta::minutes(1));
    let second = setup.reminder.tick().await.unwrap();

    let notice = first.unwrap();
    assert_eq!(notice.title, "2 action items due today");
    assert!(notice.body.ends_with(" · 1 overdue"), "{}", notice.body);
    assert_eq!(second, None);
    assert_eq!(setup.shown.0.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn before_nine_nothing_and_a_sleeping_mac_catches_up_until_noon() {
    let early = setup(-TimeDelta::minutes(1)).await;
    let late_morning = setup(TimeDelta::hours(2)).await;
    let afternoon = setup(TimeDelta::hours(3)).await;

    assert_eq!(early.reminder.tick().await.unwrap(), None);
    assert!(late_morning.reminder.tick().await.unwrap().is_some());
    assert_eq!(afternoon.reminder.tick().await.unwrap(), None);
}

#[tokio::test]
async fn the_next_morning_reminds_again() {
    let setup = setup(TimeDelta::zero()).await;
    setup.reminder.tick().await.unwrap();
    setup.world.clock.advance(TimeDelta::days(1));
    let me = Some(setup.world.me);
    setup
        .world
        .item(Spec {
            title: "Weekend prep",
            assignee: me,
            due: Some("2026-10-10"),
            ..Spec::default()
        })
        .await;

    let saturday = setup.reminder.tick().await.unwrap().unwrap();

    assert_eq!(saturday.title, "1 action item due today");
    assert!(
        saturday.body.starts_with("Weekend prep"),
        "{}",
        saturday.body
    );
}

#[tokio::test]
async fn a_switched_off_reminder_shows_nothing() {
    let setup = setup(TimeDelta::zero()).await;
    setup.switch.0.store(false, Ordering::SeqCst);

    assert_eq!(setup.reminder.tick().await.unwrap(), None);
    assert!(setup.shown.0.lock().unwrap().is_empty());
}

#[test]
fn the_body_names_two_items_and_counts_the_rest() {
    let view = |title: &str| ActionItemView {
        title: title.into(),
        ..crate::action_items::test_world::view()
    };
    let digest = TodayDigest {
        overdue: Vec::new(),
        due_today: vec![view("A"), view("B"), view("C"), view("D")],
    };

    assert_eq!(
        notice(&digest),
        Some(Notice {
            title: "4 action items due today".into(),
            body: "A, B and 2 more".into()
        })
    );
    assert_eq!(notice(&TodayDigest::default()), None);
}
