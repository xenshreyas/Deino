//! The transcript router: where both lanes' text becomes stored segments.
//!
//! Lanes deliver text on their own threads; everything goes through one
//! channel to one router thread ([`router`]), so the holdback, the drafts
//! and the database writes need no locks:
//!
//! - **Drafts.** An open segment's committed text is saved as a row (a
//!   draft) about once a second, so `kill -9` loses at most a few seconds
//!   of transcript; when the segment closes, the same row gets its final
//!   text and words.
//! - **Bleed.** Me segments go through the holdback ([`crate::bleed`]) while
//!   echo is being cancelled; echo is discarded, never saved.
//! - **Batches.** Finals and drafts are saved together, one transaction per
//!   [`router::FLUSH`] ([`SegmentStore`]), so the UI gets one `DbChanged`
//!   per batch.
//! - **Live events.** Every change also goes to the UI as a [`SegmentUpdate`](crate::events::SegmentUpdate).

mod drafts;
mod router;
mod store;

use std::sync::mpsc::{Sender, SyncSender};

use deino_stt::{FinalSegment, LiveText, SessionSink};
pub use router::{Router, RouterParts};
pub use store::{SegmentStore, StoreError, WriterStore};

use crate::echo::EchoMode;
use crate::lane::Lane;
use crate::vad::VoiceChange;

/// What the router receives.
pub(crate) enum RouterInput {
    /// A lane's open segment changed.
    Live(Lane, LiveText),
    /// A lane closed a segment.
    Final(Lane, FinalSegment),
    /// A lane's voice detector changed state.
    Voice(Lane, VoiceChange),
    /// Echo is now cancelled, or not needed.
    Mode(EchoMode),
    /// Judge everything held, save everything, reply, and stop.
    Finish(SyncSender<()>),
}

/// Sends to the router; cloned into every lane.
#[derive(Clone, Debug)]
pub struct RouterHandle {
    inputs: Sender<RouterInput>,
}

impl RouterHandle {
    fn send(&self, input: RouterInput) {
        // A closed router has already finished the recording; late input is moot.
        self.inputs.send(input).ok();
    }

    /// A lane's voice detector changed state.
    pub fn voice(&self, lane: Lane, change: VoiceChange) {
        self.send(RouterInput::Voice(lane, change));
    }

    /// Echo handling changed.
    pub fn echo_mode(&self, mode: EchoMode) {
        self.send(RouterInput::Mode(mode));
    }

    /// The `SessionSink` a lane's transcriber delivers into.
    pub fn sink(&self, lane: Lane) -> Box<dyn SessionSink> {
        Box::new(LaneSink {
            lane,
            router: self.clone(),
        })
    }
}

/// Forwards one lane's text to the router.
struct LaneSink {
    lane: Lane,
    router: RouterHandle,
}

impl SessionSink for LaneSink {
    fn live(&mut self, text: &LiveText) {
        self.router.send(RouterInput::Live(self.lane, text.clone()));
    }

    fn segment(&mut self, segment: FinalSegment) {
        self.router.send(RouterInput::Final(self.lane, segment));
    }
}
