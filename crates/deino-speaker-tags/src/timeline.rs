//! [`Timeline`]: polls of the active speaker, sampled on the recording's
//! clock, turned into [`TagSpan`]s.
//!
//! The indicator changed somewhere between two polls, so a span starts and
//! ends **halfway between** the poll that last saw the old state and the one
//! that first saw the new (an error of at most half the poll interval).
//! When polls stop (a pause, a stalled read, the end of the recording) the
//! open span closes half an interval after its last sighting: the timeline
//! never claims time nobody looked at.

use std::time::Duration;

use deino_domain::{CallApp, DisplayName, Millis, TagSpan};

/// How often the watcher looks at the meeting window (the card's 250 ms).
pub const POLL_INTERVAL: Duration = Duration::from_millis(250);

/// Polls further apart than this are not continuous: the watcher stalled.
pub const MAX_POLL_GAP: Millis = Millis(1_000);

/// Half a poll interval, in milliseconds.
const HALF_POLL: u32 = 125;

/// Who speaks, as one poll saw it: a remote participant in one app.
pub type Sighting = (CallApp, DisplayName);

/// The span being drawn.
#[derive(Clone, Debug)]
struct Open {
    sighting: Sighting,
    start: Millis,
    seen: Millis,
}

/// Collects spans from polls (see the module docs).
#[derive(Clone, Debug, Default)]
pub struct Timeline {
    open: Option<Open>,
    closed: Vec<TagSpan>,
    last_poll: Option<Millis>,
}

impl Timeline {
    /// Records one poll at `at`: who was speaking, if a remote participant was.
    pub fn observe(&mut self, at: Millis, sighting: Option<Sighting>) {
        let continuous = self
            .last_poll
            .filter(|last| at.0.saturating_sub(last.0) <= MAX_POLL_GAP.0);
        if let Some(open) = self.open.take() {
            match continuous {
                Some(_) if Some(&open.sighting) == sighting.as_ref() => {
                    self.open = Some(Open { seen: at, ..open });
                    self.last_poll = Some(at);
                    return;
                }
                Some(last) => self.close(open, midpoint(last, at)),
                None => self.close_after_last_sighting(open),
            }
        }
        self.open = sighting.map(|sighting| Open {
            sighting,
            start: continuous.map_or(at, |last| midpoint(last, at)),
            seen: at,
        });
        self.last_poll = Some(at);
    }

    /// Polls stop for now (a pause): the open span ends after its last
    /// sighting, and the next poll starts afresh.
    pub fn interrupt(&mut self) {
        if let Some(open) = self.open.take() {
            self.close_after_last_sighting(open);
        }
        self.last_poll = None;
    }

    /// The spans closed since the last call, in time order.
    pub fn take_closed(&mut self) -> Vec<TagSpan> {
        std::mem::take(&mut self.closed)
    }

    fn close_after_last_sighting(&mut self, open: Open) {
        let end = Millis(open.seen.0.saturating_add(HALF_POLL));
        self.close(open, end);
    }

    fn close(&mut self, open: Open, end: Millis) {
        if end <= open.start {
            return;
        }
        let (app, name) = open.sighting;
        self.closed.push(TagSpan {
            start: open.start,
            end,
            name,
            app,
        });
    }
}

fn midpoint(earlier: Millis, later: Millis) -> Millis {
    Millis(earlier.0 + later.0.saturating_sub(earlier.0) / 2)
}

#[cfg(test)]
#[path = "timeline_tests.rs"]
mod tests;
