//! `ModelService` over the fake byte source and a small test catalog.

use std::sync::Arc;

use sha2::{Digest, Sha256};
use tempfile::TempDir;

use super::{
    Catalog, FakeByteSource, FakeServer, ModelError, ModelFileId, ModelService, ModelServiceParts,
    ModelState, ModelStore, Quant, RecordedModelEvents,
};

const GIB: u64 = 1 << 30;

fn content() -> Vec<u8> {
    (0..20_000_u32).map(|i| (i % 241) as u8).collect()
}

/// A catalog whose two files are both `content()`.
fn catalog() -> Catalog {
    let bytes = content();
    let sha = super::Sha256Digest::from_bytes(Sha256::digest(&bytes).into());
    let file = |quant: &str| {
        format!(
            r#"{{"filename":"tiny-{quant}.gguf","quant":"{quant}","size_bytes":{},"sha256":"{sha}"}}"#,
            bytes.len()
        )
    };
    let json = format!(
        r#"{{"models":[{{"repo":"test/tiny","revision":"abc","name":"Tiny","license":"MIT",
        "default_quant":"Q8_0","files":[{},{}]}}]}}"#,
        file("Q5_K_M"),
        file("Q8_0")
    );
    Catalog::parse(&json).unwrap()
}

struct Harness {
    _dir: TempDir,
    service: ModelService,
    source: Arc<FakeByteSource>,
    events: Arc<RecordedModelEvents>,
}

fn harness(server: FakeServer, memory_bytes: u64) -> Harness {
    let dir = tempfile::tempdir().unwrap();
    let source = Arc::new(FakeByteSource::new(content(), server));
    let events = Arc::new(RecordedModelEvents::default());
    let service = ModelService::new(ModelServiceParts {
        catalog: catalog(),
        store: ModelStore::new(dir.path().join("models")),
        source: source.clone(),
        events: events.clone(),
        memory_bytes,
    });
    Harness {
        _dir: dir,
        service,
        source,
        events,
    }
}

fn q8() -> ModelFileId {
    ModelFileId::of_filename("tiny-Q8_0.gguf")
}

#[tokio::test]
async fn the_list_recommends_the_quant_that_suits_the_memory() {
    let big = harness(FakeServer::Ranged, 64 * GIB);
    let small = harness(FakeServer::Ranged, 8 * GIB);

    let big = big.service.list().await.unwrap();
    let small = small.service.list().await.unwrap();

    assert!(big.iter().all(|file| file.state == ModelState::Missing));
    let recommended = |list: &[super::ModelFileView]| {
        list.iter()
            .find(|file| file.recommended)
            .map(|file| file.quant)
    };
    assert_eq!(recommended(&big), Some(Quant::Q8));
    assert_eq!(recommended(&small), Some(Quant::Q5KM));
}

#[tokio::test]
async fn a_download_makes_the_file_ready_and_announces_its_progress() {
    let Harness {
        service, events, ..
    } = harness(FakeServer::Ranged, 64 * GIB);

    service.download(&q8()).await.unwrap();

    let path = service.ready_path(&q8()).await.unwrap();
    assert_eq!(std::fs::read(path).unwrap(), content());
    assert_eq!(events.changed(), [q8(), q8()], "started, then finished");
    assert_eq!(events.progress().last().unwrap().downloaded_bytes, 20_000);
}

#[tokio::test]
async fn a_setting_resolves_by_the_file_id_or_the_gguf_file_name() {
    let Harness { service, .. } = harness(FakeServer::Ranged, 64 * GIB);
    service.download(&q8()).await.unwrap();

    let by_name = service
        .ready_path(&ModelFileId::from_setting("tiny-Q8_0.gguf"))
        .await;
    let by_id = service
        .ready_path(&ModelFileId::from_setting("tiny-Q8_0"))
        .await;

    assert_eq!(by_name.unwrap(), by_id.unwrap());
}

#[tokio::test]
async fn a_missing_file_has_no_ready_path() {
    let Harness { service, .. } = harness(FakeServer::Ranged, 64 * GIB);

    let missing = service.ready_path(&q8()).await;

    assert!(matches!(missing, Err(ModelError::NotDownloaded(_))));
}

/// The state of `id` in the service's list.
async fn state_of(service: &ModelService, id: &ModelFileId) -> ModelState {
    let listed = service.list().await.unwrap();
    listed
        .into_iter()
        .find(|file| &file.id == id)
        .unwrap()
        .state
}

/// Yields until `id` is downloading and has `bytes` on disk.
async fn until_downloading(service: &ModelService, id: &ModelFileId, bytes: u64) {
    let wanted = ModelState::Downloading {
        downloaded_bytes: bytes,
    };
    while state_of(service, id).await != wanted {
        tokio::task::yield_now().await;
    }
}

#[tokio::test]
async fn a_second_download_of_the_same_file_is_refused_and_cancel_keeps_the_partial() {
    let Harness {
        service, source, ..
    } = harness(FakeServer::DropsAfter(5000), 64 * GIB);
    service.download(&q8()).await.unwrap_err();
    source.set_server(FakeServer::Stalls);
    let id = q8();

    let (first, second) = tokio::join!(service.download(&id), async {
        until_downloading(&service, &id, 5000).await;
        let second = service.download(&id).await;
        service.cancel(&id);
        second
    });

    assert!(matches!(second, Err(ModelError::AlreadyDownloading(_))));
    assert!(matches!(first, Err(ModelError::Cancelled)));
    let partial = ModelState::Partial {
        downloaded_bytes: 5000,
    };
    assert_eq!(state_of(&service, &id).await, partial);
}

#[tokio::test]
async fn delete_removes_the_file_and_unknown_ids_are_not_found() {
    let Harness { service, .. } = harness(FakeServer::Ranged, 64 * GIB);
    service.download(&q8()).await.unwrap();

    service.delete(&q8()).await.unwrap();

    assert!(matches!(
        service.ready_path(&q8()).await,
        Err(ModelError::NotDownloaded(_))
    ));
    let unknown = ModelFileId::of_filename("nope.gguf");
    assert!(matches!(
        service.download(&unknown).await,
        Err(ModelError::UnknownModel(_))
    ));
}
