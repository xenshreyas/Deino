//! [`HttpDownloader`]: the [`Downloader`] over a [`NetClient`].

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use futures_util::StreamExt;
use tokio::fs::File;
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio_util::sync::CancellationToken;

use super::part::PartFile;
use super::{DownloadError, DownloadProgress, DownloadRequest, Downloaded, Downloader};
use crate::{BodyStream, NetClient, Request, Response, Status};

/// How long a download may go without receiving a byte.
pub const DEFAULT_STALL_TIMEOUT: Duration = Duration::from_secs(30);

/// Downloads over a [`NetClient`]; in the app, a scoped model-host client.
pub struct HttpDownloader {
    client: Arc<dyn NetClient>,
    stall_timeout: Duration,
}

/// What the server offers for the requested range.
enum Opened {
    /// The file's bytes from `start` on.
    Body { start: u64, response: Response },
    /// Nothing: the partial file already holds the whole file.
    Complete,
}

/// One call to [`Downloader::download`].
struct Job<'a> {
    request: &'a DownloadRequest,
    progress: &'a dyn DownloadProgress,
    cancel: &'a CancellationToken,
}

/// One response body being written to the partial file.
struct Transfer<'a> {
    job: &'a Job<'a>,
    /// The offset the body starts at.
    start: u64,
    /// The whole file's size, when known.
    total: Option<u64>,
}

impl HttpDownloader {
    /// A downloader over `client`, with [`DEFAULT_STALL_TIMEOUT`].
    pub fn new(client: Arc<dyn NetClient>) -> Self {
        Self {
            client,
            stall_timeout: DEFAULT_STALL_TIMEOUT,
        }
    }

    /// Replaces the stall timeout.
    #[must_use]
    pub fn with_stall_timeout(mut self, stall_timeout: Duration) -> Self {
        self.stall_timeout = stall_timeout;
        self
    }

    /// Asks for the file from `offset` and interprets the answer.
    async fn open(&self, request: &DownloadRequest, offset: u64) -> Result<Opened, DownloadError> {
        let mut http = Request::get(&request.url)?.timeout(None);
        if offset > 0 {
            http = http.header("range", format!("bytes={offset}-"));
        }
        let response = self.client.send(http.clone()).await?;
        match response.status {
            Status::OK => Ok(Opened::Body { start: 0, response }),
            Status::PARTIAL_CONTENT if range_start(&response) == Some(offset) => {
                Ok(Opened::Body { start: offset, response })
            }
            Status::PARTIAL_CONTENT => Err(DownloadError::BadRange),
            Status::RANGE_NOT_SATISFIABLE if unsatisfied_total(&response) == Some(offset) => {
                Ok(Opened::Complete)
            }
            Status::RANGE_NOT_SATISFIABLE => self.restart(http).await,
            status => Err(DownloadError::Status(status)),
        }
    }

    /// Asks for the whole file again, after a range the server cannot serve.
    async fn restart(&self, mut http: Request) -> Result<Opened, DownloadError> {
        http.headers.retain(|name| name != "range");
        let response = self.client.send(http).await?;
        if response.status != Status::OK {
            return Err(DownloadError::Status(response.status));
        }
        Ok(Opened::Body { start: 0, response })
    }

    /// Fetches what the partial file lacks; returns the offset the transfer
    /// resumed from (0 when the server sent the whole file again).
    async fn fetch(&self, part: &PartFile, offset: u64, job: &Job<'_>) -> Result<u64, DownloadError> {
        let opened = tokio::select! {
            biased;
            () = job.cancel.cancelled() => return Err(DownloadError::Cancelled),
            opened = self.open(job.request, offset) => opened?,
        };
        let Opened::Body { start, response } = opened else {
            return Ok(offset);
        };
        let total = job
            .request
            .size
            .or_else(|| Some(start + response.content_length()?));
        let transfer = Transfer { job, start, total };
        self.receive(part, response.body, &transfer).await?;
        Ok(start)
    }

    /// Writes `body` from the transfer's start, flushing whatever arrived
    /// even when the transfer fails, so the next attempt can resume.
    async fn receive(
        &self,
        part: &PartFile,
        body: BodyStream,
        transfer: &Transfer<'_>,
    ) -> Result<(), DownloadError> {
        let mut writer = part.writer(transfer.start).await?;
        let pumped = self.pump(body, &mut writer, transfer).await;
        writer.flush().await?;
        writer.get_ref().sync_all().await?;
        if matches!(pumped, Err(DownloadError::TooLarge { .. })) {
            part.discard().await?;
        }
        pumped
    }

    async fn pump(
        &self,
        mut body: BodyStream,
        writer: &mut BufWriter<File>,
        transfer: &Transfer<'_>,
    ) -> Result<(), DownloadError> {
        let (request, mut received) = (transfer.job.request, transfer.start);
        loop {
            let next = tokio::select! {
                biased;
                () = transfer.job.cancel.cancelled() => return Err(DownloadError::Cancelled),
                next = tokio::time::timeout(self.stall_timeout, body.next()) => next,
            };
            let chunk = match next {
                Err(_) => return Err(DownloadError::Stalled),
                Ok(None) => return Ok(()),
                Ok(Some(chunk)) => chunk?,
            };
            received += chunk.len() as u64;
            if received > request.max_bytes {
                return Err(DownloadError::TooLarge {
                    max: request.max_bytes,
                });
            }
            writer.write_all(&chunk).await?;
            transfer.job.progress.progress(received, transfer.total);
        }
    }
}

#[async_trait]
impl Downloader for HttpDownloader {
    async fn download(
        &self,
        request: &DownloadRequest,
        progress: &dyn DownloadProgress,
        cancel: &CancellationToken,
    ) -> Result<Downloaded, DownloadError> {
        let part = PartFile::of(&request.destination);
        let offset = part.resume_offset(request).await?;
        let job = Job {
            request,
            progress,
            cancel,
        };
        // A partial file of exactly the pinned size needs no request at all.
        let resumed_from = if request.size == Some(offset) {
            offset
        } else {
            self.fetch(&part, offset, &job).await?
        };
        let bytes = part.verify(request).await?;
        part.commit().await?;
        Ok(Downloaded {
            path: request.destination.clone(),
            bytes,
            resumed_from,
        })
    }
}

/// The first byte of a `206`: `Content-Range: bytes <start>-<end>/<total>`.
fn range_start(response: &Response) -> Option<u64> {
    let range = response.header("content-range")?.strip_prefix("bytes ")?;
    range.split_once('-')?.0.trim().parse().ok()
}

/// The size a `416` reports: `Content-Range: bytes */<total>`.
fn unsatisfied_total(response: &Response) -> Option<u64> {
    let range = response.header("content-range")?.strip_prefix("bytes */")?;
    range.trim().parse().ok()
}
