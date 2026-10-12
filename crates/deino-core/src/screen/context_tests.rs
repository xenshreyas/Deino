//! The stage's rules over fakes: each snapshot is read once, a missing
//! picture reads as no text, descriptions only when on and never of a
//! snapshot showing a secret.

use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use deino_domain::Millis;
use deino_testkit::at;

use super::*;
use crate::screen::{FakeScreenRepo, NewSnapshot, ScreenRepo, ScreenTexts};

const MEETING: &str = "meeting-slides";

/// OCR from a table of file names; anything else fails.
#[derive(Default)]
struct TableOcr {
    texts: HashMap<String, String>,
    reads: AtomicUsize,
    unsupported: bool,
}

impl ScreenOcr for TableOcr {
    fn read(&self, png: &Path) -> Result<String, OcrError> {
        self.reads.fetch_add(1, Ordering::SeqCst);
        if self.unsupported {
            return Err(OcrError::Unsupported);
        }
        let name = png.file_name().and_then(|name| name.to_str()).unwrap_or("");
        self.texts
            .get(name)
            .cloned()
            .ok_or_else(|| OcrError::Failed("no such picture".to_owned()))
    }
}

/// Describes every picture the same way, when on.
struct Describer {
    on: bool,
    calls: Mutex<Vec<usize>>,
}

#[async_trait]
impl SnapshotDescriber for Describer {
    async fn enabled(&self) -> bool {
        self.on
    }

    async fn describe(&self, png: Vec<u8>) -> Result<String, DescribeError> {
        self.calls.lock().unwrap().push(png.len());
        Ok("  A slide with a revenue chart.  ".to_owned())
    }
}

/// Hides `sk-ant-` keys.
struct KeyRedactor;

#[async_trait]
impl TextRedactor for KeyRedactor {
    async fn redact(&self, texts: Vec<String>) -> Vec<String> {
        texts
            .into_iter()
            .map(|text| text.replace("sk-ant-123", "[redacted]"))
            .collect()
    }
}

struct World {
    repo: Arc<FakeScreenRepo>,
    ocr: Arc<TableOcr>,
    describer: Arc<Describer>,
    service: ScreenContextService,
    _dir: tempfile::TempDir,
}

async fn world(ocr: TableOcr, describe: bool) -> World {
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path().join(MEETING).join(SNAPSHOTS_DIR);
    std::fs::create_dir_all(&folder).unwrap();
    let repo = Arc::new(FakeScreenRepo::default());
    for (at_ms, name) in [(0, "0000000000.png"), (12_000, "0000012000.png"), (30_000, "0000030000.png")] {
        std::fs::write(folder.join(name), b"png bytes").unwrap();
        let snapshot = NewSnapshot {
            meeting: MeetingId::new(MEETING),
            at: Millis(at_ms),
            file_name: name.to_owned(),
            width: 1_920,
            height: 1_080,
            phash: 0,
        };
        repo.add_snapshot(&snapshot, at("2026-10-12T09:00:00Z")).await.unwrap();
    }
    let ocr = Arc::new(ocr);
    let describer = Arc::new(Describer { on: describe, calls: Mutex::default() });
    let service = ScreenContextService::new(ScreenContextParts {
        repo: repo.clone(),
        ocr: ocr.clone(),
        describer: describer.clone(),
        redactor: Arc::new(KeyRedactor),
        recordings: dir.path().to_path_buf(),
    });
    World { repo, ocr, describer, service, _dir: dir }
}

fn slides() -> TableOcr {
    let texts = [
        ("0000000000.png", "Q3 Review\n\n  Revenue up 40%  "),
        ("0000012000.png", "API key: sk-ant-123"),
    ];
    TableOcr {
        texts: texts.map(|(name, text)| (name.to_owned(), text.to_owned())).into(),
        ..TableOcr::default()
    }
}

#[tokio::test]
async fn each_snapshot_is_read_once_and_a_missing_picture_reads_as_no_text() {
    let world = world(slides(), false).await;
    let meeting = MeetingId::new(MEETING);

    let report = world.service.read(&meeting).await.unwrap();
    assert_eq!(report, ScreenContextReport { read: 3, described: 0 });
    let again = world.service.read(&meeting).await.unwrap();
    assert_eq!(again.read, 0, "a re-run reads only new snapshots");
    assert_eq!(world.ocr.reads.load(Ordering::SeqCst), 3);

    let texts = world.repo.screen_texts(&meeting).await.unwrap();
    let read: Vec<(Millis, &str)> = texts.iter().map(|t| (t.at, t.text.as_str())).collect();
    assert_eq!(
        read,
        [(Millis(0), "Q3 Review\nRevenue up 40%"), (Millis(12_000), "API key: sk-ant-123")]
    );
}

#[tokio::test]
async fn without_text_recognition_the_stage_says_so() {
    let ocr = TableOcr { unsupported: true, ..TableOcr::default() };
    let world = world(ocr, false).await;

    let error = world.service.read(&MeetingId::new(MEETING)).await.unwrap_err();
    assert!(matches!(error, ScreenContextError::Unsupported));
    assert_eq!(error.code(), ErrorCode::Unavailable);
}

#[tokio::test]
async fn descriptions_skip_a_snapshot_showing_a_secret() {
    let world = world(slides(), true).await;

    let report = world.service.read(&MeetingId::new(MEETING)).await.unwrap();

    assert_eq!(report.described, 2, "the key's snapshot is never sent");
    assert_eq!(world.describer.calls.lock().unwrap().len(), 2);
    assert_eq!(world.repo.description(1).as_deref(), Some("A slide with a revenue chart."));
    assert_eq!(world.repo.description(2), None);
}

#[test]
fn text_is_trimmed_and_capped() {
    assert_eq!(clean(" a \n\n b "), "a\nb");
    assert_eq!(clean(&"x".repeat(MAX_TEXT_CHARS + 10)).len(), MAX_TEXT_CHARS);
}
