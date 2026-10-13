//! Logging: to stderr, and on macOS also to
//! `~/Library/Logs/com.xenshreyas.deino/deino.log`, so what happened on the
//! owner's Mac can be read afterwards (Console.app shows the folder too).
//!
//! The logger starts before Tauri, so the folder is built from `HOME`; it is
//! the folder Tauri's `app_log_dir` resolves to. The file is moved to
//! `deino.log.1` once it passes 10 MB, at launch. `RUST_LOG` still filters
//! (default `info`). Logs never hold transcript text, notes, titles,
//! prompts or secrets (CLAUDE.md), only IDs and counts.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;

/// The size at which the log is set aside at launch.
const MAX_BYTES: u64 = 10 * 1024 * 1024;

/// Starts the logger; call once, first thing.
pub fn init() {
    let env = env_logger::Env::default().default_filter_or("info");
    let mut builder = env_logger::Builder::from_env(env);
    if let Some(file) = open_log_file() {
        builder.target(env_logger::Target::Pipe(Box::new(Tee { file })));
    }
    builder.init();
}

/// The log file, opened for appending; `None` where there is none.
fn open_log_file() -> Option<File> {
    let path = log_path()?;
    fs::create_dir_all(path.parent()?).ok()?;
    if fs::metadata(&path).is_ok_and(|meta| meta.len() > MAX_BYTES) {
        let _ignored = fs::rename(&path, path.with_extension("log.1"));
    }
    OpenOptions::new().create(true).append(true).open(path).ok()
}

/// `~/Library/Logs/com.xenshreyas.deino/deino.log` on macOS.
#[cfg(target_os = "macos")]
fn log_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(PathBuf::from(home).join("Library/Logs/com.xenshreyas.deino/deino.log"))
}

/// Elsewhere only stderr is used.
#[cfg(not(target_os = "macos"))]
fn log_path() -> Option<PathBuf> {
    None
}

/// Writes each record to stderr and to the file.
struct Tee {
    file: File,
}

impl Write for Tee {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let _ignored = io::stderr().write_all(buf);
        self.file.write_all(buf)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        let _ignored = io::stderr().flush();
        self.file.flush()
    }
}
