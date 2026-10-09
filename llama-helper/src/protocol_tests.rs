//! Pins the exact JSON of every request and response.

use serde_json::json;

use super::*;

fn parse(line: &str) -> Request {
    serde_json::from_str(line).unwrap()
}

fn line(response: &Response) -> String {
    let mut out = Vec::new();
    write_response(&mut out, response).unwrap();
    String::from_utf8(out).unwrap()
}

fn full_completion() -> CompletionRequest {
    CompletionRequest {
        prompt: "hi".into(),
        max_tokens: Some(64),
        context_size: Some(4096),
        model_path: Some("/m.gguf".into()),
        sampling: SamplingOptions {
            temperature: Some(0.5),
            top_k: Some(20),
            top_p: Some(0.75),
            presence_penalty: Some(0.25),
            frequency_penalty: Some(0.0),
            repeat_penalty: Some(1.5),
            penalty_last_n: Some(256),
        },
        stop_tokens: Some(vec!["<end_of_turn>".into()]),
    }
}

const FULL_FIELDS: &str = r#""prompt":"hi","max_tokens":64,"context_size":4096,"model_path":"/m.gguf","temperature":0.5,"top_k":20,"top_p":0.75,"presence_penalty":0.25,"frequency_penalty":0.0,"repeat_penalty":1.5,"penalty_last_n":256,"stop_tokens":["<end_of_turn>"]"#;

#[test]
fn generate_reads_every_field() {
    let request = parse(&format!(r#"{{"type":"generate",{FULL_FIELDS}}}"#));
    assert_eq!(request, Request::Generate(full_completion()));
}

#[test]
fn generate_needs_only_a_prompt() {
    let request = parse(r#"{"type":"generate","prompt":"hi"}"#);
    let expected = CompletionRequest {
        prompt: "hi".into(),
        ..CompletionRequest::default()
    };
    assert_eq!(request, Request::Generate(expected));
}

#[test]
fn stream_reads_the_same_fields_as_generate() {
    let request = parse(&format!(r#"{{"type":"stream",{FULL_FIELDS}}}"#));
    assert_eq!(request, Request::Stream(full_completion()));
}

#[test]
fn grammar_adds_a_json_schema() {
    let line = format!(
        r#"{{"type":"grammar",{FULL_FIELDS},"json_schema":{{"type":"object","properties":{{"a":{{"type":"string"}}}}}}}}"#
    );
    let expected = GrammarRequest {
        completion: full_completion(),
        json_schema: json!({"type": "object", "properties": {"a": {"type": "string"}}}),
    };
    assert_eq!(parse(&line), Request::Grammar(expected));
}

#[test]
fn grammar_without_a_schema_is_invalid() {
    let result = serde_json::from_str::<Request>(r#"{"type":"grammar","prompt":"hi"}"#);
    assert!(result.unwrap_err().to_string().contains("json_schema"));
}

#[test]
fn embed_reads_its_fields() {
    let request =
        parse(r#"{"type":"embed","model_path":"/e.gguf","inputs":["a","b"],"context_size":512}"#);
    let expected = EmbedRequest {
        model_path: "/e.gguf".into(),
        inputs: vec!["a".into(), "b".into()],
        context_size: Some(512),
    };
    assert_eq!(request, Request::Embed(expected));
}

#[test]
fn ping_and_shutdown_have_no_fields() {
    assert_eq!(parse(r#"{"type":"ping"}"#), Request::Ping);
    assert_eq!(parse(r#"{"type":"shutdown"}"#), Request::Shutdown);
}

#[test]
fn completion_response_keeps_the_legacy_shape() {
    let ok = Response::Completion {
        text: "hello".into(),
        error: None,
    };
    assert_eq!(
        line(&ok),
        "{\"type\":\"response\",\"text\":\"hello\",\"error\":null}\n"
    );
    let failed = Response::Completion {
        text: String::new(),
        error: Some("Generation failed: x".into()),
    };
    assert_eq!(
        line(&failed),
        "{\"type\":\"response\",\"text\":\"\",\"error\":\"Generation failed: x\"}\n"
    );
}

#[test]
fn token_response() {
    let token = Response::Token { text: "é".into() };
    assert_eq!(line(&token), "{\"type\":\"token\",\"text\":\"é\"}\n");
}

#[test]
fn done_response_names_each_stop_reason() {
    let reasons = [
        (StopReason::EndOfGeneration, "end_of_generation"),
        (StopReason::MaxTokens, "max_tokens"),
        (StopReason::StopToken, "stop_token"),
    ];
    for (stop, name) in reasons {
        let done = Response::Done {
            prompt_tokens: 12,
            output_tokens: 3,
            stop,
        };
        let expected = format!(
            "{{\"type\":\"done\",\"prompt_tokens\":12,\"output_tokens\":3,\"stop\":\"{name}\"}}\n"
        );
        assert_eq!(line(&done), expected);
    }
}

#[test]
fn embeddings_response() {
    let embeddings = Response::Embeddings {
        vectors: vec![vec![0.6, 0.8], vec![1.0, 0.0]],
        prompt_tokens: 7,
    };
    assert_eq!(
        line(&embeddings),
        "{\"type\":\"embeddings\",\"vectors\":[[0.6,0.8],[1.0,0.0]],\"prompt_tokens\":7}\n"
    );
}

#[test]
fn control_responses() {
    assert_eq!(line(&Response::Pong), "{\"type\":\"pong\"}\n");
    assert_eq!(line(&Response::Goodbye), "{\"type\":\"goodbye\"}\n");
    let error = Response::Error {
        message: "Invalid request: x".into(),
    };
    assert_eq!(
        line(&error),
        "{\"type\":\"error\",\"message\":\"Invalid request: x\"}\n"
    );
}
