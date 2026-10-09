//! The worker's process plumbing: the protocol pipe, the exit path and the
//! stderr logger. The only `unsafe` in the crate lives here, each block a
//! single libc call with its safety argument.

use std::io::{self, Write};

use log::{LevelFilter, Log, Metadata, Record};

use super::LOG_LEVEL_ENV;

/// Prefix on worker log lines, so the supervisor can tell them from raw
/// native output (such as a `GGML_ASSERT` message right before an abort).
pub const LOG_LINE_PREFIX: char = '\u{1}';

/// The writer the protocol uses: the original stdout.
pub type ProtocolOut = Box<dyn Write + Send>;

/// Takes the real stdout for the protocol and points fd 1 at stderr, so
/// native code that prints to stdout cannot corrupt a frame.
#[cfg(unix)]
#[allow(
    unsafe_code,
    reason = "fd duplication has no safe std API; each call is checked"
)]
pub fn take_stdout_for_protocol() -> io::Result<ProtocolOut> {
    use std::os::fd::FromRawFd;

    // SAFETY: `dup` has no preconditions; the result is checked below.
    let fd = unsafe { libc::dup(libc::STDOUT_FILENO) };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: both descriptors are open for the life of the process.
    if unsafe { libc::dup2(libc::STDERR_FILENO, libc::STDOUT_FILENO) } < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: `fd` is a fresh descriptor that nothing else owns.
    Ok(Box::new(unsafe { std::fs::File::from_raw_fd(fd) }))
}

/// Elsewhere stdout is used as is (Windows and Linux are compile-only targets
/// for Deino); a stray native print there would corrupt the protocol.
#[cfg(not(unix))]
pub fn take_stdout_for_protocol() -> io::Result<ProtocolOut> {
    Ok(Box::new(io::stdout()))
}

/// Ends the process now, skipping C++ static destructors (ggml-metal asserts
/// in its teardown while a model is alive); the OS reclaims the memory.
#[cfg(unix)]
#[allow(
    unsafe_code,
    reason = "`_exit` is the only way to skip C++ static destructors"
)]
pub fn exit_now(code: i32) -> ! {
    let _flushed = io::stderr().flush();
    // SAFETY: `_exit` only ends the process; nothing runs afterwards.
    unsafe { libc::_exit(code) }
}

/// Ends the process now.
#[cfg(not(unix))]
pub fn exit_now(code: i32) -> ! {
    std::process::exit(code)
}

/// Runs the worker's main thread (and the decoder threads it starts) at
/// user-initiated quality of service. The transducer decoder runs on the CPU, so CPU contention,
/// not the GPU, drives tail latency (STT spike, §5.2).
#[cfg(target_os = "macos")]
#[allow(unsafe_code, reason = "setting the thread QoS class has no safe API")]
pub fn raise_priority() {
    // SAFETY: only changes the calling thread's scheduling class.
    let status = unsafe {
        libc::pthread_set_qos_class_self_np(libc::qos_class_t::QOS_CLASS_USER_INITIATED, 0)
    };
    if status != 0 {
        log::warn!("could not raise the transcription worker's QoS ({status})");
    }
}

/// Thread QoS is a macOS concept; elsewhere the default priority stays.
#[cfg(not(target_os = "macos"))]
pub fn raise_priority() {}

/// Writes one line to stderr before the logger exists.
pub fn report(message: &str) {
    let _ignored = writeln!(io::stderr(), "{message}");
}

/// Installs the stderr logger at the level the app passed down.
pub fn install_logger() {
    static LOGGER: StderrLogger = StderrLogger;
    let level = std::env::var(LOG_LEVEL_ENV)
        .ok()
        .and_then(|level| level.parse::<LevelFilter>().ok())
        .unwrap_or(LevelFilter::Info);
    if log::set_logger(&LOGGER).is_ok() {
        log::set_max_level(level);
    }
}

/// Logs to stderr as `\x01LEVEL\ttarget\tmessage` lines.
struct StderrLogger;

impl Log for StderrLogger {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        metadata.level() <= log::max_level()
    }

    fn log(&self, record: &Record<'_>) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let message = record.args().to_string();
        let mut stderr = io::stderr().lock();
        for line in message.lines() {
            let level = record.level();
            let target = record.target();
            let _ignored = writeln!(stderr, "{LOG_LINE_PREFIX}{level}\t{target}\t{line}");
        }
    }

    fn flush(&self) {
        let _ignored = io::stderr().flush();
    }
}
