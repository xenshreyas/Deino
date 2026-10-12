//! The start observer port: who hears that a recording started.
//!
//! [`RecordingService`](crate::RecordingService) calls its observer once a
//! recording runs and its meeting row exists. The app links the calendar
//! event nearest to now (ORG-03) and may rename the meeting after it, and
//! starts reading the meeting app's speaker names (CAP-05); the recorder
//! then shows the title it was given. The recorder never knows who listens;
//! [`StartObservers`] tells several.

use std::sync::Arc;

use async_trait::async_trait;
use deino_domain::MeetingId;

/// Hears that a recording started.
#[async_trait]
pub trait StartObserver: Send + Sync {
    /// Called once `meeting` is recording. `titled` is true when the caller
    /// named the meeting, so an observer should keep that name. Returns the
    /// meeting's new title when the observer renamed it.
    ///
    /// The recording is already running and cannot be refused, so an
    /// observer handles and logs its own failures instead of returning them.
    async fn started(&self, meeting: &MeetingId, titled: bool) -> Option<String>;
}

/// Nobody listens: tests and builds without calendars.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoStartObserver;

#[async_trait]
impl StartObserver for NoStartObserver {
    async fn started(&self, _meeting: &MeetingId, _titled: bool) -> Option<String> {
        None
    }
}

/// Several observers, each told of every start, in order. The first title
/// one of them gives wins.
pub struct StartObservers(Vec<Arc<dyn StartObserver>>);

impl StartObservers {
    /// Tells `observers`, in this order.
    pub fn new(observers: Vec<Arc<dyn StartObserver>>) -> Self {
        Self(observers)
    }
}

#[async_trait]
impl StartObserver for StartObservers {
    async fn started(&self, meeting: &MeetingId, titled: bool) -> Option<String> {
        let mut title = None;
        for observer in &self.0 {
            let given = observer.started(meeting, titled).await;
            title = title.or(given);
        }
        title
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    /// Counts the starts it hears, and answers with `title`.
    struct Told {
        title: Option<&'static str>,
        told: Mutex<usize>,
    }

    impl Told {
        fn answering(title: Option<&'static str>) -> Arc<Self> {
            Arc::new(Self {
                title,
                told: Mutex::new(0),
            })
        }
    }

    #[async_trait]
    impl StartObserver for Told {
        async fn started(&self, _meeting: &MeetingId, _titled: bool) -> Option<String> {
            *self.told.lock().unwrap() += 1;
            self.title.map(str::to_owned)
        }
    }

    #[tokio::test]
    async fn every_observer_hears_a_start_and_the_first_title_wins() {
        let silent = Told::answering(None);
        let first = Told::answering(Some("Weekly sync"));
        let second = Told::answering(Some("Other"));
        let all = StartObservers::new(vec![silent.clone(), first.clone(), second.clone()]);

        let title = all.started(&MeetingId::new("meeting"), false).await;

        assert_eq!(title.as_deref(), Some("Weekly sync"));
        for observer in [silent, first, second] {
            assert_eq!(*observer.told.lock().unwrap(), 1);
        }
    }
}
