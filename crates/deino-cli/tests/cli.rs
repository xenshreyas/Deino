//! `deino` against a seeded sample database: every command's Markdown and
//! JSON output (snapshots), exports, and failures with their exit codes.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "test helpers fail loudly, like the tests they serve"
)]

#[path = "support/sample.rs"]
mod sample;

use sample::Sample;

#[tokio::test]
async fn meetings_list_newest_first_in_markdown_and_json() {
    let sample = Sample::new().await;

    insta::assert_snapshot!("meetings", sample.stdout(&["meetings", "-n", "3"]));
    insta::assert_snapshot!(
        "meetings_json",
        sample.stdout(&["--json", "meetings", "--tag", "customer"])
    );
}

#[tokio::test]
async fn meetings_filter_by_folder_person_company_and_days() {
    let sample = Sample::new().await;
    let ids = |args: &[&str]| -> Vec<String> {
        let json: serde_json::Value =
            serde_json::from_str(&sample.stdout(&[&["--json", "meetings"], args].concat()))
                .unwrap();
        json["meetings"]
            .as_array()
            .unwrap()
            .iter()
            .map(|meeting| meeting["id"].as_str().unwrap().to_owned())
            .collect()
    };

    assert_eq!(
        ids(&["--folder", "Customers"]),
        [
            "meeting-fixture-customer-support",
            "meeting-fixture-sales-discovery"
        ]
    );
    assert_eq!(
        ids(&["--company", "brightkite-logistics.com"]),
        ["meeting-fixture-sales-discovery"]
    );
    assert_eq!(
        ids(&[
            "--person",
            "Maya Chen",
            "--from",
            "2026-10-06",
            "--to",
            "2026-10-08"
        ]),
        ["meeting-fixture-standup", "meeting-fixture-one-on-one"]
    );
}

#[tokio::test]
async fn reading_meetings_brings_notes_items_and_files() {
    let sample = Sample::new().await;

    insta::assert_snapshot!(
        "meeting_read",
        sample.stdout(&["meetings", "read", "meeting-fixture-standup"])
    );
    let json = sample.stdout(&[
        "--json",
        "meetings",
        "read",
        "meeting-fixture-one-on-one",
        "meeting-fixture-interview",
    ]);
    let json: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(json["meetings"].as_array().unwrap().len(), 2);
    assert!(json["meetings"][0]["myNotes"].as_str().unwrap().len() > 10);
}

#[tokio::test]
async fn transcripts_print_every_line_or_a_page() {
    let sample = Sample::new().await;

    let all = sample.stdout(&["transcripts", "read", "meeting-fixture-standup"]);
    let page = sample.stdout(&[
        "transcripts",
        "read",
        "meeting-fixture-standup",
        "--from-ms",
        "60000",
        "-n",
        "3",
    ]);

    assert_eq!(all.lines().filter(|line| line.starts_with('[')).count(), 58);
    assert!(!all.contains("More follows"));
    insta::assert_snapshot!("transcript_page", page);
}

#[tokio::test]
async fn search_action_items_and_the_directory_render_compactly() {
    let sample = Sample::new().await;

    insta::assert_snapshot!("search", sample.stdout(&["search", "release", "cut"]));
    insta::assert_snapshot!(
        "action_items_mine",
        sample.stdout(&["action-items", "--assignee", "me", "--status", "any"])
    );
    insta::assert_snapshot!(
        "action_items_undated_json",
        sample.stdout(&[
            "--json",
            "action-items",
            "--assignee",
            "Priya",
            "--due",
            "no_date",
            "-n",
            "2",
        ])
    );
    insta::assert_snapshot!("people", sample.stdout(&["people", "brightkite"]));
    insta::assert_snapshot!("companies", sample.stdout(&["companies"]));
    insta::assert_snapshot!("tags", sample.stdout(&["tags"]));
    insta::assert_snapshot!("folders", sample.stdout(&["folders"]));
    insta::assert_snapshot!(
        "calendar",
        sample.stdout(&["calendar", "--from", "2026-10-07", "--to", "2026-10-08"])
    );
}

#[tokio::test]
async fn exports_print_text_and_write_binary_files() {
    let sample = Sample::new().await;
    let pdf = sample.dir.path().join("standup.pdf");

    let markdown = sample.stdout(&["export", "meeting-fixture-standup"]);
    let json = sample.stdout(&["export", "meeting-fixture-standup", "-f", "json"]);
    let written = sample.stdout(&[
        "--json",
        "export",
        "meeting-fixture-standup",
        "-f",
        "pdf",
        "--out",
        pdf.to_str().unwrap(),
    ]);

    assert!(
        markdown.starts_with("---\n"),
        "Markdown starts with frontmatter"
    );
    assert!(markdown.contains("# Eng standup"));
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&json).unwrap()["meeting"]["title"],
        "Eng standup"
    );
    assert!(std::fs::read(&pdf).unwrap().starts_with(b"%PDF"));
    assert!(written.contains("\"format\": \"pdf\""));
}

#[tokio::test]
async fn failures_have_codes_messages_and_exit_codes() {
    let sample = Sample::new().await;

    let missing = sample
        .deino(&["--json", "meetings", "read", "meeting-nope"])
        .assert()
        .code(3);
    let ambiguous = sample
        .deino(&["meetings", "--person", "a"])
        .assert()
        .code(4);
    let usage = sample
        .deino(&["meetings", "--from", "Tuesday"])
        .assert()
        .code(2);

    let stderr =
        |output: &std::process::Output| String::from_utf8_lossy(&output.stderr).into_owned();
    insta::assert_snapshot!("error_not_found_json", stderr(missing.get_output()));
    assert!(stderr(ambiguous.get_output()).starts_with("error: \"a\" matches several people"));
    assert!(stderr(usage.get_output()).contains("--from"));
}

#[tokio::test]
async fn a_missing_database_is_explained_and_never_created() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("deino.sqlite");

    let output = assert_cmd::Command::cargo_bin("deino-cli")
        .unwrap()
        .env("DEINO_DB", &path)
        .args(["--json", "meetings"])
        .assert()
        .code(5);

    let stderr = String::from_utf8_lossy(&output.get_output().stderr).into_owned();
    let error: serde_json::Value = serde_json::from_str(&stderr).unwrap();
    assert_eq!(error["error"]["code"], "unavailable");
    assert!(
        error["error"]["message"]
            .as_str()
            .unwrap()
            .contains("open the Deino app once")
    );
    assert!(!path.exists());
}
