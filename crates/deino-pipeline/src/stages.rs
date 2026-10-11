//! The pipeline's stages, one module each, registered in
//! [`registry`](crate::registry).
//!
//! | Order | Stage | Until |
//! |---|---|---|
//! | 10 | [`FinalizeAudioStage`] (`finalize_audio`) | |
//! | 30 | [`DiarizeStage`] (`diarize`) | |
//! | 35 | [`IdentifySpeakersStage`] (`identify_speakers`) | |
//! | 50 | [`LegacySummaryStage`] (`legacy_summary`) | NOT-02's `enhance_notes` replaces it |
//! | 60 | [`ExtractActionItemsStage`] (`extract_action_items`) | |
//! | 70 | [`AutoTagStage`] (`auto_tag`) | |
//! | 75 | [`AutoFolderStage`] (`auto_folder`) | |
//! | 99 | [`AutoExportStage`] (`auto_export`) | |
//!
//! PARITY_PLAN §4.2 lists the orders the later cards' stages take.

mod auto_export;
mod auto_folder;
#[cfg(test)]
mod auto_folder_tests;
mod auto_tag;
mod diarize;
mod extract_action_items;
mod finalize_audio;
mod identify_speakers;
mod legacy_summary;

pub use auto_export::AutoExportStage;
#[cfg(any(test, feature = "test-support"))]
pub use auto_export::ScriptedAutoExporter;
pub use auto_folder::AutoFolderStage;
#[cfg(any(test, feature = "test-support"))]
pub use auto_folder::RecordedAutoFiling;
pub use auto_tag::AutoTagStage;
#[cfg(any(test, feature = "test-support"))]
pub use auto_tag::RecordedAutoTags;
pub use diarize::DiarizeStage;
pub use extract_action_items::ExtractActionItemsStage;
pub use finalize_audio::FinalizeAudioStage;
pub use identify_speakers::IdentifySpeakersStage;
#[cfg(any(test, feature = "test-support"))]
pub use identify_speakers::RecordedIdentifications;
#[cfg(any(test, feature = "test-support"))]
pub use legacy_summary::RecordedSummaries;
pub use legacy_summary::{LegacySummaryStage, Summarizer, SummaryInput};
