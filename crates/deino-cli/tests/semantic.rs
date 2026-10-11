//! Semantic search through llama-helper: with the app's embedding model
//! beside the database and a helper to run it, `search` reports `hybrid`;
//! without either it reports `keyword`. The helper here is a stand-in script
//! that answers every `embed` line, and the model a sparse file of the
//! catalogue's exact size.

#![cfg(unix)]
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "test helpers fail loudly, like the tests they serve"
)]

#[path = "support/sample.rs"]
mod sample;

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use sample::Sample;

/// The embedding model's file name and size in the bundled catalogue.
const MODEL: (&str, u64) = ("nomic-embed-text-v1.5.Q8_0.gguf", 146_146_432);

/// A stand-in llama-helper: one embeddings line per request line.
fn fake_helper(dir: &Path) -> PathBuf {
    let path = dir.join("llama-helper");
    let script = "#!/bin/sh\nwhile read -r line; do\n  echo '{\"type\":\"embeddings\",\"vectors\":[[0.6,0.8]]}'\ndone\n";
    std::fs::write(&path, script).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path
}

/// The downloaded model, as a sparse file of the right size.
fn install_model(database: &Path) {
    let folder = database.parent().unwrap().join("models/embedding");
    std::fs::create_dir_all(&folder).unwrap();
    let file = std::fs::File::create(folder.join(MODEL.0)).unwrap();
    file.set_len(MODEL.1).unwrap();
}

fn mode(sample: &Sample, helper: Option<&Path>) -> String {
    let mut command = sample.deino(&["--json", "search", "release cut"]);
    match helper {
        Some(helper) => command.env("DEINO_LLAMA_HELPER", helper),
        None => command.env("DEINO_LLAMA_HELPER", "/nonexistent/llama-helper"),
    };
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    json["mode"].as_str().unwrap().to_owned()
}

#[tokio::test]
async fn with_the_model_and_a_helper_search_is_hybrid() {
    let sample = Sample::new().await;
    install_model(&sample.path);
    let helper = fake_helper(sample.dir.path());

    assert_eq!(mode(&sample, Some(&helper)), "hybrid");
}

#[tokio::test]
async fn without_the_helper_or_the_model_search_is_keyword_only() {
    let sample = Sample::new().await;
    let helper = fake_helper(sample.dir.path());

    let no_model = mode(&sample, Some(&helper));
    install_model(&sample.path);
    let no_helper = mode(&sample, None);

    assert_eq!(no_model, "keyword");
    assert_eq!(no_helper, "keyword");
}
