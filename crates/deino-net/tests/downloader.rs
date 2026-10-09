//! The downloader against a real HTTP server on a local socket: resume,
//! `416`, a server that ignores ranges, size and hash checks, and stalls.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "test helpers fail loudly, like the tests they serve"
)]

#[path = "support/server.rs"]
mod server;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use deino_net::{
    DownloadError, DownloadRequest, Downloaded, Downloader, HttpDownloader, ReqwestNetClient,
    Sha256Digest,
};
use sha2::{Digest, Sha256};
use server::{Reply, Server};
use tempfile::TempDir;
use tokio_util::sync::CancellationToken;

/// The file every test downloads.
const MODEL: &[u8] = b"GGUF weights: 0123456789abcdefghijklmnopqrstuvwxyz";

/// Short, so the stall test finishes quickly; real downloads use 30 s.
const STALL: Duration = Duration::from_millis(300);

fn digest_of(bytes: &[u8]) -> Sha256Digest {
    Sha256Digest::from_bytes(Sha256::digest(bytes).into())
}

struct Fixture {
    _dir: TempDir,
    destination: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let dir = TempDir::new().unwrap();
        let destination = dir.path().join("models").join("model.gguf");
        Self {
            _dir: dir,
            destination,
        }
    }

    fn part(&self) -> PathBuf {
        self.destination.with_file_name("model.gguf.part")
    }

    fn with_part(self, bytes: &[u8]) -> Self {
        std::fs::create_dir_all(self.destination.parent().unwrap()).unwrap();
        std::fs::write(self.part(), bytes).unwrap();
        self
    }

    fn request(&self, server: &Server, size: Option<u64>) -> DownloadRequest {
        DownloadRequest {
            url: format!("{}/resolve/main/model.gguf?download=true", server.base),
            destination: self.destination.clone(),
            size,
            max_bytes: 1024,
            sha256: Some(digest_of(MODEL)),
        }
    }
}

async fn download(request: &DownloadRequest) -> Result<Downloaded, DownloadError> {
    let client = Arc::new(ReqwestNetClient::new().unwrap());
    let downloader = HttpDownloader::new(client).with_stall_timeout(STALL);
    let progress = |_received: u64, _total: Option<u64>| {};
    downloader
        .download(request, &progress, &CancellationToken::new())
        .await
}

fn read(path: &Path) -> Vec<u8> {
    std::fs::read(path).unwrap()
}

#[tokio::test]
async fn a_fresh_download_is_verified_and_moved_into_place() {
    let fixture = Fixture::new();
    let server = Server::start(vec![Reply::new(200, MODEL)]).await;

    let done = download(&fixture.request(&server, Some(MODEL.len() as u64))).await.unwrap();

    assert_eq!(read(&fixture.destination), MODEL);
    assert_eq!((done.bytes, done.resumed_from), (MODEL.len() as u64, 0));
    assert!(!fixture.part().exists());
}

#[tokio::test]
async fn a_partial_file_resumes_with_a_range_request() {
    let fixture = Fixture::new().with_part(&MODEL[..20]);
    let rest = &MODEL[20..];
    let range = format!("bytes 20-{}/{}", MODEL.len() - 1, MODEL.len());
    let server = Server::start(vec![Reply::new(206, rest).header("content-range", &range)]).await;

    let done = download(&fixture.request(&server, Some(MODEL.len() as u64))).await.unwrap();

    assert_eq!(done.resumed_from, 20);
    assert_eq!(read(&fixture.destination), MODEL);
    assert!(server.requests()[0].contains("range: bytes=20-"));
}

#[tokio::test]
async fn a_server_that_ignores_the_range_restarts_the_file() {
    let fixture = Fixture::new().with_part(b"stale bytes from another file");
    let server = Server::start(vec![Reply::new(200, MODEL)]).await;

    let done = download(&fixture.request(&server, None)).await.unwrap();

    assert_eq!(done.resumed_from, 0);
    assert_eq!(read(&fixture.destination), MODEL);
}

#[tokio::test]
async fn a_416_for_a_complete_partial_file_finishes_without_a_body() {
    let fixture = Fixture::new().with_part(MODEL);
    let total = format!("bytes */{}", MODEL.len());
    let server = Server::start(vec![Reply::new(416, b"").header("content-range", &total)]).await;

    let done = download(&fixture.request(&server, None)).await.unwrap();

    assert_eq!(done.resumed_from, MODEL.len() as u64);
    assert_eq!(read(&fixture.destination), MODEL);
}

#[tokio::test]
async fn a_416_for_a_bogus_partial_file_downloads_it_again() {
    let fixture = Fixture::new().with_part(b"not the model");
    let total = format!("bytes */{}", MODEL.len());
    let server = Server::start(vec![
        Reply::new(416, b"").header("content-range", &total),
        Reply::new(200, MODEL),
    ])
    .await;

    download(&fixture.request(&server, None)).await.unwrap();

    assert_eq!(read(&fixture.destination), MODEL);
    assert!(!server.requests()[1].contains("range:"));
}

#[tokio::test]
async fn a_hash_mismatch_deletes_the_file_and_never_reaches_the_destination() {
    let fixture = Fixture::new();
    let server = Server::start(vec![Reply::new(200, b"tampered weights")]).await;

    let result = download(&fixture.request(&server, None)).await;

    assert!(matches!(result, Err(DownloadError::ChecksumMismatch)));
    assert!(!fixture.destination.exists());
    assert!(!fixture.part().exists());
}

#[tokio::test]
async fn a_stalled_transfer_keeps_what_arrived_for_the_next_attempt() {
    let fixture = Fixture::new();
    let server = Server::start(vec![Reply::new(200, MODEL).stall_after(10)]).await;

    let result = download(&fixture.request(&server, Some(MODEL.len() as u64))).await;

    assert!(matches!(result, Err(DownloadError::Stalled)));
    assert_eq!(read(&fixture.part()), &MODEL[..10]);
    assert!(!fixture.destination.exists());
}

#[tokio::test]
async fn a_body_past_the_size_cap_is_refused_and_discarded() {
    let fixture = Fixture::new();
    let server = Server::start(vec![Reply::new(200, MODEL)]).await;
    let request = DownloadRequest {
        max_bytes: 16,
        ..fixture.request(&server, None)
    };

    let result = download(&request).await;

    assert!(matches!(result, Err(DownloadError::TooLarge { max: 16 })));
    assert!(!fixture.part().exists());
}

#[tokio::test]
async fn a_short_file_is_a_size_mismatch_and_stays_for_a_resume() {
    let fixture = Fixture::new();
    let server = Server::start(vec![Reply::new(200, &MODEL[..30])]).await;

    let result = download(&fixture.request(&server, Some(MODEL.len() as u64))).await;

    assert!(matches!(
        result,
        Err(DownloadError::SizeMismatch { actual: 30, .. })
    ));
    assert_eq!(read(&fixture.part()).len(), 30);
}

#[tokio::test]
async fn a_pinned_size_partial_file_needs_no_request() {
    let fixture = Fixture::new().with_part(MODEL);
    let server = Server::start(Vec::new()).await;

    download(&fixture.request(&server, Some(MODEL.len() as u64))).await.unwrap();

    assert!(server.requests().is_empty());
    assert_eq!(read(&fixture.destination), MODEL);
}
