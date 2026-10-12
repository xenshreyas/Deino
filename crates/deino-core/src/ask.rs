//! "Ask about this meeting" (AI-02-lite, D13): single-meeting questions and
//! recipes (AI-04), answered from the meeting's notes and transcript with
//! transcript citations, streamed, and kept as a short per-meeting thread.
//!
//! The full cross-meeting agent (AI-02) stays parked (D13); this reads one
//! meeting through `enhance::MaterialSources`, so NOT-09's redactor hides
//! secrets in the transcript, and this module passes the notes through the
//! same redactor. The model answers in text with `[s12]` markers (the
//! transcript's short references, ADR 0030); `answer.rs` resolves them to
//! real segments and drops the rest, so every stored citation names a
//! segment of this meeting. The UI opens them with NOT-03's excerpts.
//!
//! | File | Role |
//! |---|---|
//! | `model.rs` | [`AskMessage`], [`AnswerPart`], [`AnswerCitation`], [`AskRequest`] |
//! | `answer.rs` | [`parse_answer`]: `[s12]` markers to resolved citations |
//! | `prompt.rs` | [`AskContext`]: the material, redacted and rendered, and the `chat` prompt |
//! | `answerer.rs` | [`AskModel`]: streams an answer; [`GatewayAskModel`] over route `chat` |
//! | `repo.rs` | [`AskStore`], [`SqliteAskRepo`]: the thread, pruned to [`MAX_THREAD_MESSAGES`] |
//! | `service.rs` | [`AskService`]: ask, preview, read and clear a thread |

mod answer;
mod answerer;
mod error;
mod model;
mod prompt;
mod repo;
mod service;
#[cfg(test)]
mod service_tests;

pub use answer::parse_answer;
pub use answerer::{AskModel, GatewayAskModel, NoAskModel};
pub use error::AskError;
pub use model::{AnswerCitation, AnswerPart, AskMessage, AskRequest, AskRole, RecipeLabel};
pub use prompt::{AskContext, HISTORY_TURNS, MAX_TRANSCRIPT_CHARS};
pub use repo::{AskStore, MAX_THREAD_MESSAGES, SqliteAskRepo};
pub use service::{AskDeps, AskService};
