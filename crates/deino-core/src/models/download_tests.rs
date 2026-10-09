//! The resumable download against a fake byte source: every recovery rule.

use std::path::{Path, PathBuf};
use std::time::Duration;

use sha2::{Digest, Sha256};
use tempfile::TempDir;
use tokio_util::sync::CancellationToken;

use super::download::{Control, Download, Limits, fetch, partial_path};
use super::{FakeByteSource, FakeServer, FetchError, ModelError, Sha256Digest};

/// The file every test downloads.
fn content() -> Vec<u8> {
    (0..10_000_u32).map(|i| (i % 251) as u8).collect()
}

fn digest(bytes: &[u8]) -> Sha256Digest {
    Sha256Digest::from_bytes(Sha256::digest(bytes).into())
}

struct Job {
    _dir: TempDir,
    dest: PathBuf,
    sha256: Sha256Digest,
    size: u64,
}

impl Job {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("model.gguf");
        let content = content();
        Self {
            sha256: digest(&content),
            size: content.len() as u64,
            dest,
            _dir: dir,
        }
    }

    fn partial(&self) -> PathBuf {
        partial_path(&self.dest)
    }

    async fn run(
        &self,
        source: &FakeByteSource,
        cancel: &CancellationToken,
    ) -> Result<Vec<u64>, ModelError> {
        let job = Download {
            url: "https://example.invalid/model.gguf",
            dest: &self.dest,
            size: self.size,
            sha256: self.sha256,
        };
        let mut seen = Vec::new();
        let mut progress = |bytes| seen.push(bytes);
        let mut control = Control {
            limits: Limits {
                connect: Duration::from_secs(1),
                stall: Duration::from_secs(1),
            },
            cancel,
            progress: &mut progress,
        };
        fetch(source, &job, &mut control).await?;
        Ok(seen)
    }
}

fn read(path: &Path) -> Vec<u8> {
    std::fs::read(path).unwrap()
}

#[tokio::test]
async fn a_fresh_download_is_verified_then_renamed() {
    let job = Job::new();
    let source = FakeByteSource::new(content(), FakeServer::Ranged);

    let progress = job.run(&source, &CancellationToken::new()).await.unwrap();

    assert_eq!(read(&job.dest), content());
    assert!(!job.partial().exists());
    assert_eq!(progress.first(), Some(&0));
    assert_eq!(progress.last(), Some(&job.size));
}

#[tokio::test]
async fn a_partial_download_resumes_from_its_last_byte() {
    let job = Job::new();
    std::fs::write(job.partial(), &content()[..4000]).unwrap();
    let source = FakeByteSource::new(content(), FakeServer::Ranged);

    job.run(&source, &CancellationToken::new()).await.unwrap();

    assert_eq!(source.requested_offsets(), [4000]);
    assert_eq!(read(&job.dest), content());
}

#[tokio::test]
async fn a_server_that_ignores_the_range_restarts_the_file_instead_of_appending() {
    let job = Job::new();
    std::fs::write(job.partial(), &content()[..4000]).unwrap();
    let source = FakeByteSource::new(content(), FakeServer::IgnoresRanges);

    job.run(&source, &CancellationToken::new()).await.unwrap();

    assert_eq!(read(&job.dest), content());
}

#[tokio::test]
async fn a_dropped_connection_keeps_the_partial_and_the_next_try_resumes() {
    let job = Job::new();
    let source = FakeByteSource::new(content(), FakeServer::DropsAfter(3000));

    let dropped = job.run(&source, &CancellationToken::new()).await;
    assert!(matches!(
        dropped,
        Err(ModelError::Network(FetchError::Connection(_)))
    ));
    assert_eq!(read(&job.partial()).len(), 3000);

    source.set_server(FakeServer::Ranged);
    job.run(&source, &CancellationToken::new()).await.unwrap();

    assert_eq!(source.requested_offsets(), [0, 3000]);
    assert_eq!(read(&job.dest), content());
}

#[tokio::test]
async fn a_complete_partial_is_verified_without_asking_the_server() {
    let job = Job::new();
    std::fs::write(job.partial(), content()).unwrap();
    let source = FakeByteSource::new(content(), FakeServer::Ranged);

    job.run(&source, &CancellationToken::new()).await.unwrap();

    assert!(source.requested_offsets().is_empty());
    assert!(job.dest.exists());
}

#[tokio::test]
async fn an_oversized_partial_is_deleted_and_downloaded_again() {
    let job = Job::new();
    std::fs::write(job.partial(), vec![7_u8; 12_000]).unwrap();
    let source = FakeByteSource::new(content(), FakeServer::Ranged);

    job.run(&source, &CancellationToken::new()).await.unwrap();

    assert_eq!(source.requested_offsets(), [0]);
    assert_eq!(read(&job.dest), content());
}

#[tokio::test]
async fn corrupt_bytes_fail_the_hash_and_are_deleted() {
    let job = Job::new();
    let mut corrupt = content();
    corrupt[5000] ^= 0xff;
    let source = FakeByteSource::new(corrupt, FakeServer::Ranged);

    let failed = job.run(&source, &CancellationToken::new()).await;

    assert!(matches!(failed, Err(ModelError::HashMismatch)));
    assert!(!job.partial().exists());
    assert!(!job.dest.exists());
}

#[tokio::test]
async fn a_server_sending_too_much_is_cut_off_and_its_bytes_deleted() {
    let job = Job::new();
    let mut longer = content();
    longer.extend([0_u8; 500]);
    let source = FakeByteSource::new(longer, FakeServer::Ranged);

    let failed = job.run(&source, &CancellationToken::new()).await;

    assert!(matches!(
        failed,
        Err(ModelError::SizeMismatch {
            expected: 10_000,
            ..
        })
    ));
    assert!(!job.partial().exists());
}

#[tokio::test(start_paused = true)]
async fn a_stalled_transfer_times_out_and_keeps_its_partial() {
    let job = Job::new();
    std::fs::write(job.partial(), &content()[..2000]).unwrap();
    let source = FakeByteSource::new(content(), FakeServer::Stalls);

    let stalled = job.run(&source, &CancellationToken::new()).await;

    assert!(matches!(
        stalled,
        Err(ModelError::Network(FetchError::Stalled))
    ));
    assert_eq!(read(&job.partial()).len(), 2000);
}

#[tokio::test]
async fn a_cancel_stops_the_transfer_and_keeps_its_partial() {
    let job = Job::new();
    std::fs::write(job.partial(), &content()[..2000]).unwrap();
    let source = FakeByteSource::new(content(), FakeServer::Stalls);
    let cancel = CancellationToken::new();
    cancel.cancel();

    let cancelled = job.run(&source, &cancel).await;

    assert!(matches!(cancelled, Err(ModelError::Cancelled)));
    assert_eq!(read(&job.partial()).len(), 2000);
}
