//! Models: the transcription models Deino can download, and their files.
//!
//! [`ModelService`] lists the bundled [`Catalog`]'s files with their state on
//! this Mac, recommends the quant that suits its memory (`Q8_0` above 16 GB,
//! `Q5_K_M` otherwise), and downloads files with resume and SHA-256
//! verification on every path. Bytes come through the [`ByteSource`] port:
//! the app's adapter over `deino-net`'s model-download client (FND-12). The STT
//! engine (`deino-stt`) never sees this module, only the verified file path,
//! so transcription has no route to the network (§4.0).

mod catalog;
pub mod download;
mod error;
mod events;
#[cfg(any(test, feature = "test-support"))]
mod fake;
mod model;
mod service;
mod source;
mod store;

pub use catalog::Catalog;
pub use error::ModelError;
pub use events::ModelEvents;
#[cfg(any(test, feature = "test-support"))]
pub use fake::{FakeByteSource, FakeServer, RecordedModelEvents};
pub use model::{
    CatalogFile, CatalogModel, DownloadProgress, ModelFileId, ModelFileView, ModelState, Quant,
    Sha256Digest,
};
pub use service::{ModelService, ModelServiceParts};
pub use source::{ByteSource, ByteStream, FetchError};
pub use store::ModelStore;

#[cfg(test)]
mod download_tests;
#[cfg(test)]
mod service_tests;
