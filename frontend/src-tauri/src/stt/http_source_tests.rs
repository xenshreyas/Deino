//! The HTTP adapter and the resumable download against a local socket
//! server: full downloads, resumes, servers that ignore `Range`, failures.

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use deino_core::models::download::{Control, Download, Limits, fetch, partial_path};
use deino_core::models::{FetchError, ModelError, Sha256Digest};
use sha2::{Digest, Sha256};
use tokio_util::sync::CancellationToken;

use super::http_source::NetByteSource;

/// A source over the root `deino-net` client: the test server is on the
/// loopback, which the app's model-download scope would refuse.
fn test_source() -> NetByteSource {
    NetByteSource::new(Arc::new(crate::composition::root_client().unwrap()))
}

/// How the server answers one connection.
#[derive(Clone, Copy, Debug)]
enum Answer {
    /// Honours `Range` with a 206.
    Ranged,
    /// Sends the whole file with a 200 whatever was asked.
    IgnoresRange,
    /// Promises the whole file, sends this many bytes, then hangs up.
    HangsUpAfter(usize),
    /// 404.
    Missing,
}

fn content() -> Vec<u8> {
    (0..50_000_u32).map(|i| (i % 253) as u8).collect()
}

/// A one-file server answering successive connections as `answers` says.
struct Server {
    url: String,
    ranges: Arc<Mutex<Vec<Option<String>>>>,
}

impl Server {
    fn start(answers: Vec<Answer>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/model.gguf", listener.local_addr().unwrap());
        let ranges = Arc::new(Mutex::new(Vec::new()));
        let seen = Arc::clone(&ranges);
        thread::spawn(move || {
            for answer in answers {
                let (stream, _) = listener.accept().unwrap();
                let range = serve(stream, answer);
                seen.lock().unwrap().push(range);
            }
        });
        Self { url, ranges }
    }

    fn ranges(&self) -> Vec<Option<String>> {
        self.ranges.lock().unwrap().clone()
    }
}

/// Answers one request; returns the `Range` header it carried.
fn serve(mut stream: TcpStream, answer: Answer) -> Option<String> {
    let mut range = None;
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        if line.trim().is_empty() {
            break;
        }
        if let Some(value) = line.to_ascii_lowercase().strip_prefix("range: bytes=") {
            range = Some(value.trim().to_string());
        }
    }
    let body = content();
    let offset: usize = range
        .as_deref()
        .and_then(|r| r.trim_end_matches('-').parse().ok())
        .unwrap_or(0);
    let response = match answer {
        Answer::Missing => "HTTP/1.1 404 Not Found\r\ncontent-length: 0\r\n\r\n"
            .as_bytes()
            .to_vec(),
        Answer::Ranged if offset > 0 => {
            let head = format!(
                "HTTP/1.1 206 Partial Content\r\ncontent-length: {}\r\ncontent-range: bytes {offset}-{}/{}\r\n\r\n",
                body.len() - offset,
                body.len() - 1,
                body.len()
            );
            [head.into_bytes(), body[offset..].to_vec()].concat()
        }
        Answer::Ranged | Answer::IgnoresRange => ok(&body, body.len()),
        Answer::HangsUpAfter(count) => ok(&body, count),
    };
    // The client may hang up first (the interrupted-download case).
    let _written = stream.write_all(&response);
    range
}

fn ok(body: &[u8], send: usize) -> Vec<u8> {
    let head = format!("HTTP/1.1 200 OK\r\ncontent-length: {}\r\n\r\n", body.len());
    [head.into_bytes(), body[..send].to_vec()].concat()
}

struct Target {
    _dir: tempfile::TempDir,
    dest: PathBuf,
}

impl Target {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("model.gguf");
        Self { _dir: dir, dest }
    }

    async fn download(&self, url: &str) -> Result<(), ModelError> {
        let body = content();
        let job = Download {
            url,
            dest: &self.dest,
            size: body.len() as u64,
            sha256: Sha256Digest::from_bytes(Sha256::digest(&body).into()),
        };
        let cancel = CancellationToken::new();
        let mut progress = |_bytes| {};
        let mut control = Control {
            limits: Limits {
                connect: Duration::from_secs(5),
                stall: Duration::from_secs(5),
            },
            cancel: &cancel,
            progress: &mut progress,
        };
        fetch(&test_source(), &job, &mut control).await
    }
}

#[tokio::test]
async fn a_full_download_is_verified_and_given_its_name() {
    let server = Server::start(vec![Answer::Ranged]);
    let target = Target::new();

    target.download(&server.url).await.unwrap();

    assert_eq!(std::fs::read(&target.dest).unwrap(), content());
    assert_eq!(server.ranges(), [None]);
}

#[tokio::test]
async fn an_interrupted_download_resumes_with_a_range_request() {
    let server = Server::start(vec![Answer::HangsUpAfter(20_000), Answer::Ranged]);
    let target = Target::new();

    let interrupted = target.download(&server.url).await;
    assert!(matches!(
        interrupted,
        Err(ModelError::Network(FetchError::Connection(_)))
    ));
    let kept = std::fs::metadata(partial_path(&target.dest)).unwrap().len();
    target.download(&server.url).await.unwrap();

    assert!(kept > 0);
    assert_eq!(server.ranges()[1], Some(format!("{kept}-")));
    assert_eq!(std::fs::read(&target.dest).unwrap(), content());
}

#[tokio::test]
async fn a_server_that_ignores_the_range_restarts_the_file() {
    let server = Server::start(vec![Answer::IgnoresRange]);
    let target = Target::new();
    std::fs::write(partial_path(&target.dest), &content()[..10_000]).unwrap();

    target.download(&server.url).await.unwrap();

    assert_eq!(server.ranges(), [Some("10000-".to_string())]);
    assert_eq!(std::fs::read(&target.dest).unwrap(), content());
}

#[tokio::test]
async fn a_missing_file_is_an_http_status_error() {
    let server = Server::start(vec![Answer::Missing]);
    let target = Target::new();

    let missing = target.download(&server.url).await;

    assert!(matches!(
        missing,
        Err(ModelError::Network(FetchError::Status(404)))
    ));
}

/// The real path end to end: Hugging Face, the pinned revision, resume and
/// SHA-256 verification, on the smallest quant (477 MB). Needs the network:
/// `cargo test -p deino --lib real_hugging_face -- --ignored --nocapture`.
#[tokio::test]
#[ignore = "downloads 477 MB from Hugging Face"]
async fn real_hugging_face_download_is_verified() {
    use std::sync::Arc;

    use deino_core::models::{
        Catalog, ModelService, ModelServiceParts, ModelStore, Quant, RecordedModelEvents,
    };

    let dir = tempfile::tempdir().unwrap();
    let catalog = Catalog::bundled().unwrap();
    let id = catalog
        .live_model()
        .unwrap()
        .file(Quant::Q4KM)
        .unwrap()
        .id
        .clone();
    let service = ModelService::new(ModelServiceParts {
        catalog,
        store: ModelStore::new(dir.path()),
        source: Arc::new(test_source()),
        events: Arc::new(RecordedModelEvents::default()),
        memory_bytes: 8 << 30,
    });

    service.download(&id).await.unwrap();

    let path = service.ready_path(&id).await.unwrap();
    assert_eq!(std::fs::metadata(path).unwrap().len(), 477_274_496);
}
