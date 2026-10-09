//! Wire tests for Ollama's native API: models, context windows, pulls, deletes.

use std::sync::Arc;

use futures_util::StreamExt;
use serde_json::json;

use super::*;
use crate::{FakeHttp, HttpError, StaticCredentials};

fn ollama(http: FakeHttp) -> (OllamaProvider, Arc<FakeHttp>) {
    let http = Arc::new(http);
    let credentials = Arc::new(StaticCredentials::new());
    (
        OllamaProvider::new(http.clone(), credentials.clone(), credentials),
        http,
    )
}

#[tokio::test]
async fn models_come_from_api_tags_with_size_and_date() {
    let tags = r#"{"models":[{"name":"gemma3:1b","model":"gemma3:1b","modified_at":"2026-10-01T10:00:00Z","size":815319791}]}"#;
    let (provider, http) = ollama(FakeHttp::new().reply(200, tags));

    let models = provider
        .models(&crate::DraftConnection::default())
        .await
        .unwrap();

    assert_eq!(models[0].id, "gemma3:1b");
    assert_eq!(models[0].size_bytes, Some(815_319_791));
    assert_eq!(http.requests()[0].url, "http://localhost:11434/api/tags");
}

#[tokio::test]
async fn a_server_that_is_not_running_is_unavailable_with_advice() {
    let (provider, _) = ollama(FakeHttp::new().fail(HttpError::Connect("refused".into())));

    let error = provider
        .models(&crate::DraftConnection::default())
        .await
        .unwrap_err();

    assert!(error.to_string().contains("install or start Ollama"));
}

#[tokio::test]
async fn the_context_window_comes_from_api_show() {
    let show = r#"{"modelfile":"","details":{"family":"llama"},"model_info":{"llama.context_length":131072}}"#;
    let (provider, http) = ollama(FakeHttp::new().reply(200, show));

    let window = provider.context_window("llama3.2:3b").await;

    assert_eq!(window, Some(131_072));
    assert_eq!(
        http.json_body(0),
        json!({ "model": "llama3.2:3b", "verbose": true })
    );
}

#[tokio::test]
async fn an_unreadable_model_falls_back_to_its_family_default() {
    let (provider, _) = ollama(FakeHttp::new().reply(404, r#"{"error":"model not found"}"#));

    assert_eq!(provider.context_window("qwen2.5:7b").await, Some(8192));
}

#[tokio::test]
async fn pulls_report_percentages_then_done() {
    let lines = vec![
        b"{\"status\":\"pulling manifest\"}\n{\"status\":\"downloading\",\"completed\":50,\"total\":200}\n".to_vec(),
        b"{\"status\":\"downloading\",\"completed\":200,\"total\":200}\n{\"status\":\"success\"}\n".to_vec(),
    ];
    let (provider, http) = ollama(FakeHttp::new().reply_chunked(200, lines));

    let progress: Vec<PullProgress> = provider
        .pull("gemma3:1b")
        .await
        .unwrap()
        .map(Result::unwrap)
        .collect()
        .await;

    assert_eq!(
        progress,
        [
            PullProgress::Downloading { percent: 25 },
            PullProgress::Downloading { percent: 100 },
            PullProgress::Done
        ]
    );
    assert_eq!(
        http.json_body(0),
        json!({ "model": "gemma3:1b", "stream": true })
    );
}

#[tokio::test]
async fn a_pull_error_line_ends_the_stream_with_an_error() {
    let (provider, _) = ollama(FakeHttp::new().reply(
        200,
        "{\"error\":\"pull model manifest: file does not exist\"}\n",
    ));

    let progress: Vec<Result<PullProgress, LlmError>> =
        provider.pull("nope").await.unwrap().collect().await;

    assert!(matches!(progress[..], [Err(LlmError::Provider { .. })]));
}

#[tokio::test]
async fn delete_sends_the_model_name() {
    let (provider, http) = ollama(FakeHttp::new().reply(200, ""));

    provider.delete("gemma3:1b").await.unwrap();

    let request = &http.requests()[0];
    assert_eq!(request.method, Method::Delete);
    assert_eq!(request.url, "http://localhost:11434/api/delete");
}
