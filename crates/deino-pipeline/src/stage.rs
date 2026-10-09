//! The contract every pipeline stage implements.
//!
//! A stage is one step of post-meeting work for one meeting. The runner
//! calls [`PipelineStage::run`] with a [`StageCtx`] and reads the result:
//!
//! - `Ok(`[`StageOutcome::Done`]`)`: the work is done; the next stage runs.
//! - `Ok(`[`StageOutcome::Skipped`]`)`: nothing to do (no transcript, a
//!   feature turned off); the next stage runs, and the UI shows the reason.
//! - `Err(`[`StageError`]`)`: retryable errors are retried with backoff up
//!   to the retry limit; permanent ones stop the meeting's pipeline until
//!   the user re-runs the stage.
//!
//! Stages are modelled on spacedrive's job contract (Apache-2.0): methods,
//! not associated consts, keep the trait object-safe for
//! `Arc<dyn PipelineStage>`, and every stage must be idempotent, because a
//! crash or a re-run starts it again from the beginning.

use std::sync::Arc;

use async_trait::async_trait;
use deino_domain::{AppError, Clock, ErrorClass, ErrorCode, MeetingId, StageId};
use tokio_util::sync::CancellationToken;

use crate::ProgressSink;

/// One step of the post-meeting pipeline.
#[async_trait]
pub trait PipelineStage: Send + Sync {
    /// The stage's stable name, stored in `jobs.stage`. Never rename a stage
    /// that has shipped: its stored jobs would be pruned as unknown.
    fn id(&self) -> StageId;

    /// Where the stage runs: lower first (the table in PARITY_PLAN §4.2).
    fn order(&self) -> u16;

    /// Does the stage's work for `meeting`.
    ///
    /// Must be idempotent and should return [`StageError::Cancelled`] soon
    /// after `ctx.cancel` fires; the runner abandons a stage that does not
    /// stop within a short grace period.
    async fn run(&self, ctx: &StageCtx, meeting: MeetingId) -> Result<StageOutcome, StageError>;
}

/// What a stage receives besides the meeting: its attempt, a cancellation
/// token and the shared ports. Stage-specific ports (repositories, the LLM)
/// are given to each stage when the registry builds it.
#[derive(Clone)]
pub struct StageCtx {
    /// This attempt's number in the current run, from 1.
    pub attempt: u32,
    /// Fires when the meeting's pipeline is cancelled or the app quits.
    pub cancel: CancellationToken,
    /// The injected clock; stages never read the wall clock.
    pub clock: Arc<dyn Clock>,
    /// Where pipeline progress goes; the runner already reports each
    /// stage's status, so stages rarely need it.
    pub events: Arc<dyn ProgressSink>,
}

/// How a successful stage ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StageOutcome {
    /// The stage did its work.
    Done,
    /// The stage had nothing to do. The reason is shown to the user, so it
    /// is a fixed, user-safe sentence, never meeting content.
    Skipped(&'static str),
}

/// Why a stage failed. Messages are user-safe: they are stored in
/// `jobs.message` and shown on the meeting page.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum StageError {
    /// A failure that may pass on its own (network, rate limit, a busy
    /// database); retried with backoff.
    #[error("{0}")]
    Retryable(AppError),
    /// A failure that repeats until something changes (no model configured,
    /// invalid input); the stage waits for the user to re-run it.
    #[error("{0}")]
    Permanent(AppError),
    /// The stage stopped because it was cancelled.
    #[error("the stage was cancelled")]
    Cancelled,
}

impl StageError {
    /// A failure worth retrying.
    pub fn retryable(code: ErrorCode, message: impl Into<String>) -> Self {
        Self::Retryable(AppError::new(code, message))
    }

    /// A failure that retrying will not fix.
    pub fn permanent(code: ErrorCode, message: impl Into<String>) -> Self {
        Self::Permanent(AppError::new(code, message))
    }

    /// A module error as a retryable stage failure, keeping its code and
    /// user-safe message (storage failures, for example).
    pub fn retryable_from(error: &impl ErrorClass) -> Self {
        Self::Retryable(AppError::new(error.code(), error.to_string()))
    }

    /// True when the runner should try the stage again.
    pub fn is_retryable(&self) -> bool {
        matches!(self, Self::Retryable(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, thiserror::Error)]
    #[error("could not read the transcript")]
    struct ReadFailed;

    impl ErrorClass for ReadFailed {
        fn code(&self) -> ErrorCode {
            ErrorCode::Internal
        }
    }

    #[test]
    fn only_retryable_errors_are_retried() {
        assert!(StageError::retryable(ErrorCode::Network, "offline").is_retryable());
        assert!(!StageError::permanent(ErrorCode::Invalid, "no model").is_retryable());
        assert!(!StageError::Cancelled.is_retryable());
    }

    #[test]
    fn a_module_error_keeps_its_code_and_user_safe_message() {
        let error = StageError::retryable_from(&ReadFailed);

        assert_eq!(
            error,
            StageError::Retryable(AppError::new(
                ErrorCode::Internal,
                "could not read the transcript"
            ))
        );
        assert_eq!(error.to_string(), "could not read the transcript");
    }
}
