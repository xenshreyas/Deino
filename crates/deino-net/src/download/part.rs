//! [`PartFile`]: the partial file a download writes, and its final checks.

use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use tokio::fs::{self, File, OpenOptions};
use tokio::io::{AsyncReadExt, BufWriter};

use super::{DownloadError, DownloadRequest, Sha256Digest};

/// Bytes buffered before a write reaches the disk.
const WRITE_BUFFER: usize = 1024 * 1024;

/// Bytes read at a time while hashing.
const HASH_BUFFER: usize = 1024 * 1024;

/// `<destination>.part`: the file a download writes before it is verified.
#[derive(Debug)]
pub(super) struct PartFile {
    path: PathBuf,
    destination: PathBuf,
}

impl PartFile {
    /// The partial file of `destination`.
    pub(super) fn of(destination: &Path) -> Self {
        let mut name = destination
            .file_name()
            .map(OsString::from)
            .unwrap_or_default();
        name.push(".part");
        Self {
            path: destination.with_file_name(name),
            destination: destination.to_path_buf(),
        }
    }

    /// The bytes already on disk, or 0 when there is no partial file.
    pub(super) async fn len(&self) -> io::Result<u64> {
        match fs::metadata(&self.path).await {
            Ok(metadata) => Ok(metadata.len()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(0),
            Err(error) => Err(error),
        }
    }

    /// Where to resume: the partial file's size, or 0 after discarding one
    /// that is already too large for `request`.
    pub(super) async fn resume_offset(&self, request: &DownloadRequest) -> io::Result<u64> {
        let len = self.len().await?;
        let limit = request.size.unwrap_or(request.max_bytes).min(request.max_bytes);
        if len > limit {
            self.discard().await?;
            return Ok(0);
        }
        Ok(len)
    }

    /// Opens the file for writing from `start`: appending to what is there,
    /// or replacing it when `start` is 0.
    pub(super) async fn writer(&self, start: u64) -> io::Result<BufWriter<File>> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).await?;
        }
        let mut options = OpenOptions::new();
        if start == 0 {
            options.write(true).create(true).truncate(true);
        } else {
            options.append(true);
        }
        let file = options.open(&self.path).await?;
        Ok(BufWriter::with_capacity(WRITE_BUFFER, file))
    }

    /// Checks the finished file's size and hash. A file that is too short
    /// stays for a later resume; a longer or corrupt one is deleted.
    pub(super) async fn verify(&self, request: &DownloadRequest) -> Result<u64, DownloadError> {
        let actual = self.len().await?;
        if let Some(expected) = request.size
            && actual != expected
        {
            if actual > expected {
                self.discard().await?;
            }
            return Err(DownloadError::SizeMismatch { expected, actual });
        }
        if let Some(expected) = request.sha256
            && self.sha256().await? != expected
        {
            self.discard().await?;
            return Err(DownloadError::ChecksumMismatch);
        }
        Ok(actual)
    }

    /// Moves the verified file to its destination.
    pub(super) async fn commit(&self) -> io::Result<()> {
        fs::rename(&self.path, &self.destination).await
    }

    /// Deletes the partial file, if there is one.
    pub(super) async fn discard(&self) -> io::Result<()> {
        match fs::remove_file(&self.path).await {
            Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error),
            _ => Ok(()),
        }
    }

    async fn sha256(&self) -> io::Result<Sha256Digest> {
        let mut file = File::open(&self.path).await?;
        let mut hasher = Sha256::new();
        let mut buffer = vec![0u8; HASH_BUFFER];
        loop {
            let read = file.read(&mut buffer).await?;
            let Some(chunk) = buffer.get(..read).filter(|chunk| !chunk.is_empty()) else {
                break;
            };
            hasher.update(chunk);
        }
        Ok(Sha256Digest::from_bytes(hasher.finalize().into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_partial_file_sits_next_to_the_destination() {
        let part = PartFile::of(Path::new("/models/parakeet/model.gguf"));
        assert_eq!(part.path, Path::new("/models/parakeet/model.gguf.part"));
    }
}
