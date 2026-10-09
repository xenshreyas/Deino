//! The summary pipeline over `MockProvider`: single pass, chunking, retries.

use std::sync::Arc;

use deino_llm::{LlmError, MockProvider, ProviderId, Route, mock_gateway};

use super::*;

fn request(provider: ProviderId, transcript: String) -> SummaryRequest {
    SummaryRequest {
        transcript,
        user_context: Some("Board prep".into()),
        section_instructions: "- **Decisions**: list them".into(),
        template_markdown: "# <Add Title here>\n## Decisions".into(),
        target: Target {
            provider,
            model: "model-x".into(),
        },
    }
}

fn generator(mock: MockProvider) -> (SummaryGenerator, Arc<MockProvider>) {
    let harness = mock_gateway(Arc::new(mock));
    (
        SummaryGenerator::new(Arc::new(harness.gateway)),
        harness.mock,
    )
}

/// About 4,700 estimated tokens: more than a 4k local model takes at once.
fn long_transcript() -> String {
    "Ana: the pilot covers one region for sixty days. ".repeat(270)
}

#[tokio::test]
async fn hosted_models_fill_the_template_in_one_pass_on_the_pinned_target() {
    let answer = "<think>plan</think>\n# Brightkite pilot\n## Decisions\n- One region";
    let (generator, mock) = generator(MockProvider::new().reply(Route::NotesEnhance, answer));

    let summary = generator
        .generate(&request(ProviderId::Anthropic, long_transcript()))
        .await
        .unwrap();

    assert_eq!(
        summary.markdown,
        "# Brightkite pilot\n## Decisions\n- One region"
    );
    assert_eq!(summary.chunks, 1);
    assert!(summary.reasoning_stripped);
    let sent = mock.requests();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].model, "model-x");
    let system = sent[0].system.as_deref().unwrap();
    assert!(system.contains("- **Decisions**: list them") && system.contains("## Decisions"));
    assert!(
        sent[0].messages[0]
            .text()
            .contains("<user_context>\nBoard prep\n</user_context>")
    );
}

#[tokio::test]
async fn long_transcripts_on_local_models_are_chunked_combined_then_reported() {
    let mock = MockProvider::new()
        .reply(Route::NotesEnhance, "chunk one")
        .reply(Route::NotesEnhance, "chunk two")
        .reply(Route::NotesEnhance, "combined")
        .reply(Route::NotesEnhance, "# Title\nReport");
    let (generator, mock) = generator(mock);

    let summary = generator
        .generate(&request(ProviderId::BuiltIn, long_transcript()))
        .await
        .unwrap();

    assert_eq!(summary.markdown, "# Title\nReport");
    assert_eq!(summary.chunks, 2);
    let sent = mock.requests();
    assert_eq!(sent.len(), 4);
    assert!(sent[0].messages[0].text().contains("<transcript_chunk>"));
    assert!(
        sent[2].messages[0]
            .text()
            .contains("chunk one\n---\nchunk two")
    );
    assert!(
        sent[3].messages[0]
            .text()
            .contains("<transcript_chunks>\ncombined\n</transcript_chunks>")
    );
}

#[tokio::test]
async fn a_chunk_is_retried_once_then_the_summary_fails_with_its_number() {
    let mock = MockProvider::new()
        .reply(Route::NotesEnhance, "<think>only reasoning</think>")
        .reply(Route::NotesEnhance, "chunk one")
        .reply(Route::NotesEnhance, "")
        .reply(Route::NotesEnhance, "   ");
    let (generator, _) = generator(mock);

    let error = generator
        .generate(&request(ProviderId::Ollama, long_transcript()))
        .await
        .unwrap_err();

    assert_eq!(
        error.to_string(),
        "transcript section 2 of 2 failed: Summary chunk returned no visible summary content after reasoning removal. Please retry."
    );
}

#[tokio::test]
async fn a_missing_key_is_not_retried() {
    let mock =
        MockProvider::new().fail(Route::NotesEnhance, LlmError::NoKey(ProviderId::Anthropic));
    let (generator, mock) = generator(mock);

    let error = generator
        .generate(&request(ProviderId::Anthropic, "short".into()))
        .await
        .unwrap_err();

    assert!(matches!(error, SummaryError::Llm(LlmError::NoKey(_))));
    assert_eq!(mock.requests().len(), 1);
}

#[tokio::test]
async fn an_unterminated_reasoning_marker_fails_the_final_summary() {
    let (generator, _) =
        generator(MockProvider::new().reply(Route::NotesEnhance, "Visible\n<think>private"));

    let error = generator
        .generate(&request(ProviderId::OpenAi, "short".into()))
        .await
        .unwrap_err();

    assert!(matches!(
        error,
        SummaryError::ReasoningMarker {
            stage: "Final summary"
        }
    ));
}
