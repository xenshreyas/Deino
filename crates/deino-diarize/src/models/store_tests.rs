use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;
use deino_core::models::{ByteStream, FetchError as Transfer};
use sha2::{Digest, Sha256};

use super::*;

/// Serves fixed bytes per URL in one chunk and records the URLs asked for.
#[derive(Default)]
struct UrlSource {
    files: HashMap<&'static str, Vec<u8>>,
    requests: Mutex<Vec<String>>,
}

struct Once(Option<Vec<u8>>);

#[async_trait]
impl ByteStream for Once {
    fn start(&self) -> u64 {
        0
    }

    async fn chunk(&mut self) -> Result<Option<Vec<u8>>, Transfer> {
        Ok(self.0.take())
    }
}

#[async_trait]
impl ByteSource for UrlSource {
    async fn open(&self, url: &str, _offset: u64) -> Result<Box<dyn ByteStream>, Transfer> {
        self.requests.lock().unwrap().push(url.to_owned());
        let bytes = self.files.get(url).ok_or(Transfer::Status(404))?;
        Ok(Box::new(Once(Some(bytes.clone()))))
    }
}

/// A pinned file whose pin is the real SHA-256 of `content`.
fn pinned(name: &'static str, content: &[u8]) -> ModelFile {
    let digest = Sha256Digest::from_bytes(Sha256::digest(content).into()).to_string();
    ModelFile {
        file_name: name,
        url: Box::leak(format!("https://models.test/{name}").into_boxed_str()),
        sha256: Box::leak(digest.into_boxed_str()),
        bytes: content.len() as u64,
    }
}

fn catalog() -> Catalog {
    Catalog {
        segmentation: pinned("seg.onnx", b"segmentation weights"),
        embedding: pinned("emb.onnx", b"embedding weights"),
    }
}

fn serving(catalog: &Catalog, embedding: &[u8]) -> UrlSource {
    let mut files = HashMap::new();
    files.insert(catalog.segmentation.url, b"segmentation weights".to_vec());
    files.insert(catalog.embedding.url, embedding.to_vec());
    UrlSource {
        files,
        ..UrlSource::default()
    }
}

#[tokio::test]
async fn ensure_downloads_verifies_and_installs_every_model() {
    let dir = tempfile::tempdir().unwrap();
    let catalog = catalog();
    let store = ModelStore::with_catalog(dir.path(), catalog);
    assert!(matches!(
        store.verified_paths(),
        Err(StoreError::Missing("seg.onnx"))
    ));

    let paths = store
        .ensure(&serving(&catalog, b"embedding weights"))
        .await
        .unwrap();

    assert_eq!(
        std::fs::read(&paths.embedding).unwrap(),
        b"embedding weights"
    );
    assert_eq!(store.verified_paths().unwrap(), paths);
}

#[tokio::test]
async fn a_tampered_download_is_rejected_and_never_installed() {
    let dir = tempfile::tempdir().unwrap();
    let catalog = catalog();
    let store = ModelStore::with_catalog(dir.path(), catalog);

    let result = store.ensure(&serving(&catalog, b"embedding weightz")).await;

    let Err(StoreError::Download { file, source }) = result else {
        panic!("{result:?}");
    };
    assert_eq!(file, "emb.onnx");
    assert!(matches!(source, FetchError::HashMismatch), "{source:?}");
    assert!(!store.path(&catalog.embedding).exists());
    assert!(store.verified_paths().is_err());
}

#[tokio::test]
async fn present_models_are_not_downloaded_again() {
    let dir = tempfile::tempdir().unwrap();
    let catalog = catalog();
    let store = ModelStore::with_catalog(dir.path(), catalog);
    let source = serving(&catalog, b"embedding weights");
    store.ensure(&source).await.unwrap();
    std::fs::remove_file(store.path(&catalog.embedding)).unwrap();

    store.ensure(&source).await.unwrap();

    let requests = source.requests.lock().unwrap().clone();
    assert_eq!(
        requests.len(),
        3,
        "both once, then only the missing one: {requests:?}"
    );
    assert_eq!(
        requests.last().map(String::as_str),
        Some(catalog.embedding.url)
    );
}

#[tokio::test]
async fn a_failed_transfer_is_a_network_error() {
    let dir = tempfile::tempdir().unwrap();
    let store = ModelStore::with_catalog(dir.path(), catalog());
    let result = store.ensure(&UrlSource::default()).await;
    assert!(
        matches!(
            result,
            Err(StoreError::Download {
                file: "seg.onnx",
                ..
            })
        ),
        "{result:?}"
    );
    assert_eq!(result.unwrap_err().code(), ErrorCode::Network);
}

#[test]
fn a_file_of_the_wrong_size_is_not_present() {
    let dir = tempfile::tempdir().unwrap();
    let catalog = catalog();
    let store = ModelStore::with_catalog(dir.path(), catalog);
    std::fs::write(store.path(&catalog.segmentation), b"short").unwrap();
    assert!(!store.is_present(&catalog.segmentation).unwrap());
}
