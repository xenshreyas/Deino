//! `-[AVAssetWriter finishWritingWithCompletionHandler:]`, which cidre
//! a9587fa does not bind (only the deprecated synchronous `finishWriting`).

use std::sync::mpsc::sync_channel;
use std::time::Duration;

use block2::RcBlock;
use cidre::av;
use objc2::msg_send;
use objc2::runtime::AnyObject;

use crate::encoder::EncodeError;

/// How long finishing (encoding and emitting the last segment) may take.
const FINISH_TIMEOUT: Duration = Duration::from_secs(10);

/// Finishes `writer` and waits for its completion handler. The last partial
/// segment reaches the delegate before the handler runs.
pub(super) fn finish_writing(writer: &av::AssetWriter) -> Result<(), EncodeError> {
    let (done, finished) = sync_channel(1);
    let handler = RcBlock::new(move || {
        // The receiver may have timed out and gone; nothing else to do then.
        done.try_send(()).ok();
    });
    send_finish(writer, &handler);
    finished
        .recv_timeout(FINISH_TIMEOUT)
        .map_err(|_| EncodeError::Timeout)
}

/// Sends the message through objc2, since cidre has no binding for it.
#[allow(
    unsafe_code,
    reason = "objc2 message send to an AVAssetWriter owned by cidre"
)]
fn send_finish(writer: &av::AssetWriter, handler: &RcBlock<dyn Fn()>) {
    let object: *const AnyObject = std::ptr::from_ref(writer).cast();
    // SAFETY: `writer` is a live `AVAssetWriter` instance (cidre's type is a
    // transparent wrapper around the object pointer), the selector takes one
    // block argument and returns void, and AVFoundation copies the block.
    unsafe {
        let () = msg_send![&*object, finishWritingWithCompletionHandler: &**handler];
    }
}
