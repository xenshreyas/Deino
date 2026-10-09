//! Deino's network client: the only way anything in Deino talks to the
//! internet (FND-12, PARITY_PLAN §1 "what may leave the Mac").
//!
//! - [`NetClient`]: one required `send`, with body, JSON
//!   ([`NetClientExt`]) and Server-Sent Events ([`SseStream`]) helpers on top;
//!   [`ReqwestNetClient`] is the real one (the only `reqwest` user in the
//!   workspace) and `FakeNetClient` (feature `test-support`) the scripted one.
//! - [`ServiceId`] (from `deino-domain`) names every outside service, and
//!   [`registry`] gives each its hosts and defaults.
//! - [`Net::scoped`] hands out a [`ScopedClient`] that reaches only its
//!   service's hosts, only while the user allows the service
//!   ([`ServicePolicy`]), never in offline mode except for downloads the user
//!   started ([`Net::approved_download`]); it follows redirects itself so
//!   every hop is checked.
//! - The [`EgressLog`] records every attempt (time, service, host, method,
//!   status, bytes up and down; never a URL, header or body), in memory and,
//!   through [`EgressPersister`], in the `egress_log` table
//!   ([`SqliteEgressLog`]).
//! - [`Downloader`] / [`HttpDownloader`]: resumable, size-capped,
//!   stall-guarded, SHA-256-verified model downloads.
//! - [`NetworkControls`]: the Settings → Privacy → Network page's service.
//!
//! The transcription and audio crates must never depend on this crate; a
//! `cargo tree` check in `scripts/check-net-isolation.sh` enforces it.

mod client;
mod controls;
mod download;
mod egress;
mod error;
#[cfg(any(test, feature = "test-support"))]
mod fake;
mod host;
mod http;
mod policy;
pub mod registry;
mod reqwest_client;
mod scoped;
mod sse;

pub use client::{NetClient, NetClientExt};
pub use controls::{ControlsError, MAX_EGRESS_PAGE, NetworkControls, ServiceStatus};
pub use deino_domain::{ServiceId, ServiceKind};
pub use download::{
    DEFAULT_STALL_TIMEOUT, DownloadError, DownloadProgress, DownloadRequest, Downloaded,
    Downloader, HttpDownloader, InvalidDigest, Sha256Digest,
};
pub use egress::{
    EgressEntry, EgressLog, EgressOutcome, EgressPersister, EgressReader, EgressWriter,
    RING_CAPACITY, SqliteEgressLog,
};
pub use error::NetError;
#[cfg(any(test, feature = "test-support"))]
pub use fake::{FakeNetClient, FakeReply};
pub use host::{HostPattern, host_of};
pub use http::{
    BodyStream, DEFAULT_TIMEOUT, Headers, MAX_BODY_BYTES, Method, Request, Response, Status,
};
#[cfg(any(test, feature = "test-support"))]
pub use policy::FakeServicePolicy;
pub use policy::{Grant, ServicePolicy, ServiceSwitch, SwitchError};
pub use reqwest_client::ReqwestNetClient;
pub use scoped::{MAX_REDIRECTS, Net, NetParts, ScopedClient};
pub use sse::{SseDecoder, SseEvent, SseStream};
