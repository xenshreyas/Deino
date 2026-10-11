//! The `EventKit` thread: the one thread that owns the event store.
//!
//! `EventKit` objects are not thread-safe, and reading a month of events
//! blocks for a while, so neither may happen on the main thread or on async
//! workers. [`Worker`] owns a dedicated thread that creates the store, reads
//! it and keeps the notification observers; callers send it a [`Request`]
//! and await the reply. The thread ends when the [`Worker`] is dropped.

use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread;

use deino_core::calendar::{CalendarAccess, SourceError, SourceSnapshot, SyncTriggers, SyncWindow};
use objc2::rc::Retained;
use objc2_event_kit::EKEventStore;
use tokio::sync::oneshot;

use crate::observe::Observers;
use crate::{read, store};

/// What the `EventKit` thread is asked to do.
pub(crate) enum Request {
    /// Show the access prompt if needed; reply with the new access state.
    Prompt(oneshot::Sender<CalendarAccess>),
    /// Read the window; reply with what the store holds.
    Fetch(
        SyncWindow,
        oneshot::Sender<Result<SourceSnapshot, SourceError>>,
    ),
    /// Nudge these triggers on store changes and on wake.
    Watch(SyncTriggers),
}

/// A handle to the `EventKit` thread.
#[derive(Debug)]
pub(crate) struct Worker {
    requests: Sender<Request>,
}

impl Worker {
    /// Starts the thread. The store is created lazily, on first use.
    pub(crate) fn spawn() -> std::io::Result<Self> {
        let (requests, inbox) = channel();
        thread::Builder::new()
            .name("eventkit".into())
            .spawn(move || serve(&inbox))?;
        Ok(Self { requests })
    }

    /// Hands `request` to the thread; false when the thread is gone.
    pub(crate) fn send(&self, request: Request) -> bool {
        self.requests.send(request).is_ok()
    }
}

/// The thread's state: the store and the observers it keeps alive.
#[derive(Default)]
struct State {
    store: Option<Retained<EKEventStore>>,
    /// Whether `store` was created with access granted; one created before
    /// the grant sees no calendars until it is made again.
    store_has_access: bool,
    observers: Option<Observers>,
}

/// Serves requests until every [`Worker`] handle is dropped.
fn serve(inbox: &Receiver<Request>) {
    let mut state = State::default();
    while let Ok(request) = inbox.recv() {
        match request {
            Request::Prompt(reply) => state.request_access(reply),
            Request::Fetch(window, reply) => {
                // The caller may have timed out; nothing to do then.
                reply.send(state.fetch(window)).ok();
            }
            Request::Watch(triggers) => state.observers = Some(Observers::register(&triggers)),
        }
    }
    log::debug!(target: "calendar", "EventKit thread stopped");
}

impl State {
    /// The store, made again when access was granted since it was made.
    fn store(&mut self) -> &EKEventStore {
        let has_access = store::authorization() == CalendarAccess::Granted;
        if self.store.is_none() || (has_access && !self.store_has_access) {
            self.store = Some(store::open());
            self.store_has_access = has_access;
        }
        // Set just above when it was missing.
        self.store.get_or_insert_with(store::open)
    }

    fn request_access(&mut self, reply: oneshot::Sender<CalendarAccess>) {
        if store::authorization() != CalendarAccess::NotDetermined {
            reply.send(store::authorization()).ok();
            return;
        }
        store::request_full_access(self.store(), move |_granted| {
            reply.send(store::authorization()).ok();
        });
    }

    fn fetch(&mut self, window: SyncWindow) -> Result<SourceSnapshot, SourceError> {
        if store::authorization() != CalendarAccess::Granted {
            return Err(SourceError::AccessDenied);
        }
        Ok(read::snapshot(self.store(), window))
    }
}
