//! The pipeline's stages, one module each, registered in
//! [`registry`](crate::registry).
//!
//! | Order | Stage | Until |
//! |---|---|---|
//! | 50 | [`LegacySummaryStage`] (`legacy_summary`) | NOT-02's `enhance_notes` replaces it |
//!
//! PARITY_PLAN §4.2 lists the orders the later cards' stages take.

mod legacy_summary;

#[cfg(any(test, feature = "test-support"))]
pub use legacy_summary::RecordedSummaries;
pub use legacy_summary::{LegacySummaryStage, Summarizer, SummaryInput};
