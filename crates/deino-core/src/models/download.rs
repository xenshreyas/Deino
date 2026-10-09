//! The resumable, verified download of one model file.
//!
//! Bytes go to `<file>.partial`; the file gets its real name only after its
//! size and SHA-256 match the catalog, on every path (fresh, resumed, or a
//! partial that was already complete). Robustness rules ported from Handy's
//! `managers/model/download.rs` (MIT), in the order failures appear:
//!
//! - a partial already at full size (a crash between the last byte and the
//!   rename) is verified, not re-requested (which would loop on HTTP 416);
//!   one larger than the catalog size is deleted;
//! - opening and every chunk are bounded by [`Limits`] and race the cancel
//!   token, so a wedged transfer neither hangs forever nor ignores a cancel;
//! - a server that ignores the range and sends everything restarts the file
//!   from zero instead of appending it; a resume at any other offset is refused;
//! - a server sending more than the catalog size is cut off at the first
//!   excess byte, and the partial is deleted;
//! - a cancel, a stall or a short transfer keeps the partial for a resume;
//!   a hash mismatch deletes it.

use std::path::{Path, PathBuf};
use std::time::Duration;

use sha2::{Digest, Sha256};
use tokio::fs::{self, File, OpenOptions};
use tokio::io::AsyncWriteExt;
use tokio_util::sync::CancellationToken;

use super::{ByteSource, ByteStream, FetchError, ModelError, Sha256Digest};

/// Progress is reported each time this many more bytes have arrived.
const PROGRESS_STEP: u64 = 4 * 1024 * 1024;

/// Hashing reads the file in chunks of this size.
const HASH_CHUNK: usize = 1024 * 1024;

/// How long a transfer may wait before it counts as stalled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    /// For the server to answer.
    pub connect: Duration,
    /// For the next chunk.
    pub stall: Duration,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            connect: Duration::from_secs(15),
            stall: Duration::from_mins(1),
        }
    }
}

/// One file to fetch.
#[derive(Clone, Copy, Debug)]
pub struct Download<'a> {
    /// Where to fetch it from.
    pub url: &'a str,
    /// Where it goes once verified.
    pub dest: &'a Path,
    /// Its exact size.
    pub size: u64,
    /// Its exact content.
    pub sha256: Sha256Digest,
}

/// How one transfer is bounded, stopped and observed.
pub struct Control<'a> {
    /// Timeouts.
    pub limits: Limits,
    /// Stops the transfer, keeping its partial.
    pub cancel: &'a CancellationToken,
    /// Receives the bytes on disk as they grow.
    pub progress: &'a mut (dyn FnMut(u64) + Send),
}

/// Where the bytes of `dest` collect until they are verified.
pub fn partial_path(dest: &Path) -> PathBuf {
    let mut name = dest.file_name().unwrap_or_default().to_os_string();
    name.push(".partial");
    dest.with_file_name(name)
}

/// Fetches `job` through `source` under `control`.
pub async fn fetch(
    source: &dyn ByteSource,
    job: &Download<'_>,
    control: &mut Control<'_>,
) -> Result<(), ModelError> {
    let partial = partial_path(job.dest);
    let have = existing_len(&partial, job.size).await?;
    if have < job.size {
        let opening = source.open(job.url, have);
        let stream = bounded(control.cancel, control.limits.connect, opening).await??;
        let mut file = open_partial(&partial, stream.start(), have).await?;
        let received = copy(stream, &mut file, job, control).await?;
        if received != job.size {
            return Err(ModelError::SizeMismatch {
                expected: job.size,
                received,
            });
        }
    }
    finish(&partial, job).await
}

/// Bytes already in the partial; one longer than the file is deleted.
async fn existing_len(partial: &Path, size: u64) -> Result<u64, ModelError> {
    let have = match fs::metadata(partial).await {
        Ok(metadata) => metadata.len(),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(error) => return Err(error.into()),
    };
    if have > size {
        fs::remove_file(partial).await?;
        return Ok(0);
    }
    Ok(have)
}

/// Opens the partial at the stream's start: appending to a resume at our
/// offset, truncating when the server sends everything from zero.
async fn open_partial(partial: &Path, start: u64, have: u64) -> Result<File, ModelError> {
    let mut options = OpenOptions::new();
    options.create(true).write(true);
    if start == have && have > 0 {
        options.append(true);
    } else if start == 0 {
        options.truncate(true);
    } else {
        return Err(ModelError::Network(FetchError::Status(416)));
    }
    Ok(options.open(partial).await?)
}

/// Copies the stream into `file`; returns the bytes on disk afterwards. The
/// file is flushed on every exit, so a failed transfer resumes where it
/// really stopped.
async fn copy(
    stream: Box<dyn ByteStream>,
    file: &mut File,
    job: &Download<'_>,
    control: &mut Control<'_>,
) -> Result<u64, ModelError> {
    let copied = pump(stream, file, job, control).await;
    let flushed = file.flush().await;
    let received = copied?;
    flushed?;
    file.sync_all().await?;
    (control.progress)(received);
    Ok(received)
}

/// Moves chunks from the stream to the file, cutting off excess bytes.
async fn pump(
    mut stream: Box<dyn ByteStream>,
    file: &mut File,
    job: &Download<'_>,
    control: &mut Control<'_>,
) -> Result<u64, ModelError> {
    let mut received = stream.start();
    let mut reported = received;
    (control.progress)(received);
    while let Some(chunk) = bounded(control.cancel, control.limits.stall, stream.chunk()).await?? {
        let total = received + chunk.len() as u64;
        if total > job.size {
            if let Err(error) = fs::remove_file(partial_path(job.dest)).await {
                log::warn!(target: "models", "could not delete an oversized download: {error}");
            }
            return Err(ModelError::SizeMismatch {
                expected: job.size,
                received: total,
            });
        }
        file.write_all(&chunk).await?;
        received = total;
        if received - reported >= PROGRESS_STEP {
            reported = received;
            (control.progress)(received);
        }
    }
    Ok(received)
}

/// Runs `work` unless `cancel` fires or `limit` passes first.
async fn bounded<T>(
    cancel: &CancellationToken,
    limit: Duration,
    work: impl Future<Output = T>,
) -> Result<T, ModelError> {
    tokio::select! {
        biased;
        () = cancel.cancelled() => Err(ModelError::Cancelled),
        outcome = tokio::time::timeout(limit, work) => {
            outcome.map_err(|_elapsed| ModelError::Network(FetchError::Stalled))
        }
    }
}

/// Verifies the partial and gives it its real name; deletes it on a mismatch.
async fn finish(partial: &Path, job: &Download<'_>) -> Result<(), ModelError> {
    if sha256_of(partial).await? != job.sha256 {
        fs::remove_file(partial).await?;
        return Err(ModelError::HashMismatch);
    }
    fs::rename(partial, job.dest).await?;
    Ok(())
}

/// Hashes a file off the async runtime.
pub async fn sha256_of(path: &Path) -> Result<Sha256Digest, ModelError> {
    let path = path.to_path_buf();
    tokio::task::spawn_blocking(move || hash_file(&path))
        .await
        .map_err(|error| ModelError::Io(std::io::Error::other(error)))?
}

fn hash_file(path: &Path) -> Result<Sha256Digest, ModelError> {
    use std::io::Read;

    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0_u8; HASH_CHUNK];
    loop {
        let read = file.read(&mut buffer)?;
        let Some(bytes) = buffer.get(..read).filter(|bytes| !bytes.is_empty()) else {
            break;
        };
        hasher.update(bytes);
    }
    Ok(Sha256Digest::from_bytes(hasher.finalize().into()))
}
