//! Sensitive-information redaction (NOT-09, ADR 0038).
//!
//! With Settings → Privacy → "Hide sensitive information" on, passwords, API
//! keys, card numbers, Social Security numbers and one-time codes are
//! replaced by `[redacted]` in what reaches the notes and leaves Deino:
//!
//! - the enhanced notes' input ([`MaterialSources`](crate::enhance::MaterialSources)'s
//!   transcript), and everything else that gathers its material there;
//! - exports, auto-export and the `deino` CLI and MCP server (the
//!   [`ExportAssembler`](crate::export::ExportAssembler)'s transcript,
//!   notes and action items).
//!
//! The transcript itself keeps the originals; the user deletes them there.
//!
//! Detection is patterns first ([`Redactor::candidates`]): one
//! [`Detector`](detector::Detector) per [`SecretKind`], over typed digits
//! and spoken ones ("four two four two"). Spans whose shape proves them
//! ([`Confidence::Certain`]: a Luhn-valid card, a `sk-ant-` key, an SSN
//! written 3-2-4) are always redacted. Spans found from context alone
//! ([`Confidence::Likely`]) go to the optional [`SpanCheck`], which may only
//! reject them; [`LocalModelCheck`] runs it on the built-in local model and
//! is skipped when that would leave the Mac.
//!
//! | File | Role |
//! |---|---|
//! | `model.rs` | [`SecretKind`], [`Confidence`], [`Redaction`], [`apply`] |
//! | `words.rs`, `digits.rs` | Words with byte ranges; typed and spoken digit runs |
//! | `detector.rs`, `detectors/` | The `Detector` port and one detector per kind |
//! | `check.rs` | [`SpanCheck`]: [`NoCheck`], [`LocalModelCheck`] |
//! | `redactor.rs` | [`Redactor`], the [`TextRedactor`] port, [`SettingRedactor`] |

mod check;
mod detector;
mod detectors;
mod digits;
mod model;
mod redactor;
mod words;

#[cfg(test)]
mod acceptance_tests;
#[cfg(test)]
mod fixture_tests;

use async_trait::async_trait;

pub use check::{Candidate, LocalModelCheck, NoCheck, SpanCheck};
pub use model::{Confidence, REDACTED, Redaction, SecretKind, apply};
pub use redactor::{NoRedaction, RedactionSetting, Redactor, SettingRedactor, TextRedactor};

use crate::settings::SettingsService;

#[async_trait]
impl RedactionSetting for SettingsService {
    async fn enabled(&self) -> bool {
        self.get().await.privacy.redact_sensitive
    }
}
