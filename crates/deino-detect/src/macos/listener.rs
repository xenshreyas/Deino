//! [`CaListener`]: one Core Audio property listener that wakes the detector.
//!
//! The listener is a block with no dispatch queue, so Core Audio calls it
//! directly on its notification thread; it only calls [`Waker::wake`], which
//! never blocks. Dropping the listener removes it.

use cidre::{arc, core_audio as ca, os};

use crate::error::DetectError;
use crate::ports::Waker;

/// A registered Core Audio property listener.
pub(super) struct CaListener {
    object: ca::Obj,
    selector: ca::PropSelector,
    block: arc::R<ca::PropListenerBlock>,
}

impl CaListener {
    /// Listens to `selector` (global scope) on `object`; `what` names it in errors.
    pub(super) fn add(
        object: ca::Obj,
        selector: ca::PropSelector,
        waker: &Waker,
        what: &'static str,
    ) -> Result<Self, DetectError> {
        let waker = waker.clone();
        let mut block =
            ca::PropListenerBlock::new2(move |_count: u32, _addresses: *const ca::PropAddr| {
                waker.wake();
            });
        object
            .add_prop_listener_block(&selector.global_addr(), None, &mut block)
            .map_err(|error| DetectError::os(what, status(error)))?;
        Ok(Self {
            object,
            selector,
            block,
        })
    }
}

impl Drop for CaListener {
    fn drop(&mut self) {
        let removed = self.object.remove_prop_listener_block(
            &self.selector.global_addr(),
            None,
            &mut self.block,
        );
        if let Err(error) = removed {
            log::debug!(target: "detect", "could not remove an audio listener (OS status {})", status(error));
        }
    }
}

/// The `OSStatus` inside a cidre error.
pub(super) fn status(error: os::Error) -> i32 {
    error.0.get()
}
