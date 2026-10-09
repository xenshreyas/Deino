//! The `AVAssetWriterDelegate` that receives each CMAF segment.
//!
//! `AVFoundation` calls it on its own queue. Init and media segments are simply
//! concatenated (`init + N × (moof + mdat)`), which is a standard fragmented
//! MP4 every reader handles; [`SegmentOutput::deliver`] does the appending.

#![allow(
    clippy::transmute_ptr_to_ptr,
    reason = "cidre's define_obj_type! and add_methods macros transmute object references"
)]

use cidre::av::AssetWriterDelegate as _;
use cidre::{av, define_obj_type, ns, objc};

use crate::encoder::SegmentOutput;

define_obj_type!(
    pub(super) SegmentDelegate + av::AssetWriterDelegateImpl,
    SegmentOutput,
    DEINO_SEGMENT_DELEGATE
);

impl av::AssetWriterDelegate for SegmentDelegate {}

#[objc::add_methods]
impl av::AssetWriterDelegateImpl for SegmentDelegate {
    extern "C" fn impl_asset_writer_did_output_segment_data(
        &mut self,
        _cmd: Option<&objc::Sel>,
        _writer: &av::AssetWriter,
        segment_data: &ns::Data,
        _segment_type: av::AssetSegmentType,
    ) {
        self.inner().deliver(segment_data.as_slice());
    }
}
