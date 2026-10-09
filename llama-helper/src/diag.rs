//! Diagnostics on stderr.
//!
//! stdout carries only protocol lines, so everything a person might want to
//! read while debugging goes to stderr through [`diag!`], which the app
//! captures into its log. Diagnostics carry counts, sizes and timings only:
//! never prompt text, generated text or embeddings.

use std::fmt;

/// Writes one diagnostic line to stderr, formatted like `format!`.
macro_rules! diag {
    ($($arg:tt)*) => {
        $crate::diag::emit(::std::format_args!($($arg)*))
    };
}

pub(crate) use diag;

/// Writes `message` and a newline to stderr. Use [`diag!`] instead.
#[allow(
    clippy::print_stderr,
    reason = "the sidecar's diagnostics channel is stderr; this is its one writer"
)]
pub fn emit(message: fmt::Arguments<'_>) {
    eprintln!("{message}");
}
