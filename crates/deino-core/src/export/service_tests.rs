use std::path::PathBuf;

use super::*;
use crate::export::testing::{STANDUP, seeded_assembler};
use crate::export::{FakeVault, RecordedClipboard, RecordedOpener, ScriptedDialog};

struct Harness {
    service: ExportService,
    vault: Arc<FakeVault>,
    dialog: Arc<ScriptedDialog>,
    clipboard: Arc<RecordedClipboard>,
    opener: Arc<RecordedOpener>,
}

async fn harness(answer: Option<&str>) -> Harness {
    let (_, assembler) = seeded_assembler().await;
    let vault = Arc::new(FakeVault::default());
    let dialog = Arc::new(ScriptedDialog::answering(answer.map(PathBuf::from)));
    let clipboard = Arc::new(RecordedClipboard::default());
    let opener = Arc::new(RecordedOpener::default());
    let service = ExportService::new(ExportServiceDeps {
        assembler: Arc::new(assembler),
        vault: vault.clone(),
        save_dialog: dialog.clone(),
        clipboard: clipboard.clone(),
        opener: opener.clone(),
    });
    Harness {
        service,
        vault,
        dialog,
        clipboard,
        opener,
    }
}

fn standup() -> MeetingId {
    MeetingId::new(STANDUP)
}

#[tokio::test]
async fn saving_writes_where_the_dialog_points_and_suggests_a_dated_name() {
    let harness = harness(Some("/Users/me/Desktop/standup.pdf")).await;

    let saved = harness
        .service
        .save(&standup(), ExportFormat::Pdf, ExportSections::ALL)
        .await
        .unwrap();

    assert_eq!(
        saved,
        Some(SavedExport {
            file_name: "standup.pdf".into()
        })
    );
    let request = &harness.dialog.requests()[0];
    assert_eq!(request.file_name, "2026-10-08 Eng standup.pdf");
    assert_eq!((request.kind, request.extension), ("PDF document", "pdf"));
    let pdf = harness
        .vault
        .read("/Users/me/Desktop/standup.pdf")
        .unwrap_or_default();
    assert!(pdf.starts_with("%PDF-"));
}

#[tokio::test]
async fn a_cancelled_dialog_writes_nothing() {
    let harness = harness(None).await;

    let saved = harness
        .service
        .save(&standup(), ExportFormat::Markdown, ExportSections::ALL)
        .await
        .unwrap();

    assert_eq!(saved, None);
    assert!(harness.vault.paths().is_empty());
}

#[tokio::test]
async fn a_name_without_the_extension_gets_it_back() {
    let harness = harness(Some("/tmp/notes")).await;

    harness
        .service
        .save(&standup(), ExportFormat::Markdown, ExportSections::ALL)
        .await
        .unwrap();

    assert_eq!(harness.vault.paths(), [PathBuf::from("/tmp/notes.md")]);
}

#[tokio::test]
async fn previews_show_text_formats_as_text_and_documents_as_html() {
    let harness = harness(None).await;

    let markdown = harness
        .service
        .preview(&standup(), ExportFormat::Markdown, ExportSections::ALL)
        .await
        .unwrap();
    let pdf = harness
        .service
        .preview(&standup(), ExportFormat::Pdf, ExportSections::ALL)
        .await
        .unwrap();

    assert_eq!(markdown.kind, PreviewKind::Text);
    assert!(markdown.content.starts_with("---\ntitle: \"Eng standup\""));
    assert_eq!(markdown.file_name, "2026-10-08 Eng standup.md");
    assert_eq!(pdf.kind, PreviewKind::Html);
    assert!(pdf.content.contains("<h1"));
}

#[tokio::test]
async fn rich_copies_carry_html_and_a_plain_text_fallback() {
    let harness = harness(None).await;

    harness
        .service
        .copy(&standup(), CopyKind::RichText)
        .await
        .unwrap();
    harness
        .service
        .copy(&standup(), CopyKind::Markdown)
        .await
        .unwrap();
    harness
        .service
        .copy(&standup(), CopyKind::Transcript)
        .await
        .unwrap();

    let copies = harness.clipboard.copies();
    assert!(
        copies[0]
            .html
            .as_deref()
            .is_some_and(|html| html.contains("<h2"))
    );
    assert!(copies[0].text.starts_with("Eng standup\n"));
    assert_eq!(copies[1].html, None);
    assert!(copies[1].text.contains("## Attendees"));
    assert!(
        copies[2]
            .text
            .contains("[00:00] Shreyas: Morning, everyone.")
    );
    assert!(!copies[2].text.contains("Attendees"));
}

#[tokio::test]
async fn email_opens_a_mailto_draft() {
    let harness = harness(None).await;

    harness.service.email(&standup()).await.unwrap();

    let urls = harness.opener.urls();
    assert!(urls[0].starts_with("mailto:maya%40deino.dev,"));
    assert!(urls[0].contains("subject=Eng%20standup"));
}
