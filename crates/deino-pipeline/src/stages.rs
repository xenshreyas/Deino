//! The pipeline's stages, one module each, registered in
//! [`registry`](crate::registry).
//!
//! | Order | Stage | Card |
//! |---|---|---|
//! | 10 | [`FinalizeAudioStage`] (`finalize_audio`) | FND-05b |
//! | 20 | [`FinalTranscriptionStage`] (`final_transcription`) | CAP-08 |
//! | 30 | [`DiarizeStage`] (`diarize`) | CAP-03 |
//! | 35 | [`IdentifySpeakersStage`] (`identify_speakers`) | CAP-04 |
//! | 40 | [`VocabFixupsStage`] (`vocab_fixups`) | TRN-05 |
//! | 50 | [`EnhanceNotesStage`] (`enhance_notes`) | NOT-02 |
//! | 60 | [`ExtractActionItemsStage`] (`extract_action_items`) | AI-05 |
//! | 65 | [`AutoTitleStage`] (`auto_title`) | NOT-02 |
//! | 70 | [`AutoTagStage`] (`auto_tag`) | ORG-06 |
//! | 75 | [`AutoFolderStage`] (`auto_folder`) | ORG-05 |
//! | 80 | [`IndexStage`] (`index`) | ORG-08, AI-01 |
//! | 85 | [`InsightsStage`] (`insights`) | AI-06 |
//! | 90 | [`UpdateEntitiesStage`] (`update_entities`) | ORG-04 |
//! | 99 | [`AutoExportStage`] (`auto_export`) | EXP-02 |
//!
//! PARITY_PLAN §4.2 lists the orders the later cards' stages take.

mod auto_export;
mod auto_folder;
#[cfg(test)]
mod auto_folder_tests;
mod auto_tag;
mod auto_title;
mod diarize;
mod enhance_notes;
mod extract_action_items;
mod final_transcription;
mod finalize_audio;
mod identify_speakers;
mod index;
mod insights;
mod update_entities;
mod vocab_fixups;

pub use auto_export::AutoExportStage;
#[cfg(any(test, feature = "test-support"))]
pub use auto_export::ScriptedAutoExporter;
pub use auto_folder::AutoFolderStage;
#[cfg(any(test, feature = "test-support"))]
pub use auto_folder::RecordedAutoFiling;
pub use auto_tag::AutoTagStage;
#[cfg(any(test, feature = "test-support"))]
pub use auto_tag::RecordedAutoTags;
#[cfg(any(test, feature = "test-support"))]
pub use auto_title::RecordedTitler;
pub use auto_title::{AutoTitleStage, MeetingTitler};
pub use diarize::DiarizeStage;
pub use enhance_notes::{
    AUTOMATIC_NOTES_OFF, AutomaticNotes, EnhanceNotesStage, NotesEnhancer, stage_error,
};
#[cfg(any(test, feature = "test-support"))]
pub use enhance_notes::{FixedAutomaticNotes, RecordedEnhancer};
pub use extract_action_items::ExtractActionItemsStage;
#[cfg(any(test, feature = "test-support"))]
pub use final_transcription::FixedRepass;
pub use final_transcription::{FinalTranscriptionStage, RepassSetting};
pub use finalize_audio::FinalizeAudioStage;
pub use identify_speakers::IdentifySpeakersStage;
#[cfg(any(test, feature = "test-support"))]
pub use identify_speakers::RecordedIdentifications;
#[cfg(any(test, feature = "test-support"))]
pub use index::RecordedIndexing;
pub use index::{INDEX_STAGE, IndexStage};
pub use insights::InsightsStage;
#[cfg(any(test, feature = "test-support"))]
pub use insights::RecordedInsights;
#[cfg(any(test, feature = "test-support"))]
pub use update_entities::RecordedEntityUpdates;
pub use update_entities::UpdateEntitiesStage;
#[cfg(any(test, feature = "test-support"))]
pub use vocab_fixups::RecordedFixer;
pub use vocab_fixups::{VocabFixer, VocabFixupsStage};
