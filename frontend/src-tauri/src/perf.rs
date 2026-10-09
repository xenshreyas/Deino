//! Hot-path logging macros that compile to nothing in release builds.
//!
//! `lib.rs` declares this module first with `#[macro_use]`, so every later
//! module can call `perf_debug!` and `perf_trace!` without an import.

/// Logs at debug level in debug builds; expands to nothing in release builds.
#[cfg(debug_assertions)]
macro_rules! perf_debug {
    ($($arg:tt)*) => {
        log::debug!($($arg)*)
    };
}

/// Logs at debug level in debug builds; expands to nothing in release builds.
#[cfg(not(debug_assertions))]
macro_rules! perf_debug {
    ($($arg:tt)*) => {};
}

/// Logs at trace level in debug builds; expands to nothing in release builds.
#[cfg(debug_assertions)]
macro_rules! perf_trace {
    ($($arg:tt)*) => {
        log::trace!($($arg)*)
    };
}

/// Logs at trace level in debug builds; expands to nothing in release builds.
#[cfg(not(debug_assertions))]
macro_rules! perf_trace {
    ($($arg:tt)*) => {};
}
