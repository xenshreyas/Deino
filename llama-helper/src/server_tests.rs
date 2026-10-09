//! Drives the request loop with in-memory input and output. None of these
//! requests reaches llama.cpp: the model paths do not exist.

use super::Server;

/// The response lines for `input`, one string per line.
fn serve(input: &str) -> Vec<String> {
    let mut server = Server::new(Vec::new(), 300);
    server.run(input.as_bytes()).unwrap();
    String::from_utf8(server.out)
        .unwrap()
        .lines()
        .map(str::to_owned)
        .collect()
}

#[test]
fn answers_ping_and_skips_blank_lines() {
    assert_eq!(serve("\n  \n{\"type\":\"ping\"}\n"), [r#"{"type":"pong"}"#]);
}

#[test]
fn says_goodbye_on_shutdown_and_reads_no_further() {
    let lines = serve("{\"type\":\"shutdown\"}\n{\"type\":\"ping\"}\n");
    assert_eq!(lines, [r#"{"type":"goodbye"}"#]);
}

#[test]
fn reports_an_unparsable_request_and_carries_on() {
    let lines = serve("not json\n{\"type\":\"nope\"}\n{\"type\":\"ping\"}\n");
    assert_eq!(lines.len(), 3);
    assert!(lines[0].starts_with(r#"{"type":"error","message":"Invalid request: "#));
    assert!(lines[1].contains("unknown variant `nope`"));
    assert_eq!(lines[2], r#"{"type":"pong"}"#);
}

#[test]
fn generate_reports_a_missing_model_in_the_legacy_shape() {
    let lines = serve(r#"{"type":"generate","prompt":"hi","model_path":"/no/such.gguf"}"#);
    assert_eq!(
        lines,
        [
            r#"{"type":"response","text":"","error":"Failed to load model: unable to load model at \"/no/such.gguf\""}"#
        ]
    );
}

#[test]
fn generate_without_a_loaded_model_fails_like_before() {
    let lines = serve(r#"{"type":"generate","prompt":"hi"}"#);
    assert_eq!(
        lines,
        [r#"{"type":"response","text":"","error":"Generation failed: Model not loaded"}"#]
    );
}

#[test]
fn stream_reports_a_load_failure_as_its_terminal_error() {
    let lines = serve(r#"{"type":"stream","prompt":"hi","model_path":"/no/such.gguf"}"#);
    assert_eq!(
        lines,
        [
            r#"{"type":"error","message":"Failed to load model: unable to load model at \"/no/such.gguf\""}"#
        ]
    );
}

#[test]
fn grammar_without_a_loaded_model_is_an_error_line() {
    let lines = serve(r#"{"type":"grammar","prompt":"hi","json_schema":{"type":"object"}}"#);
    assert_eq!(
        lines,
        [r#"{"type":"error","message":"Generation failed: Model not loaded"}"#]
    );
}

#[test]
fn embed_reports_a_missing_model() {
    let lines = serve(r#"{"type":"embed","model_path":"/no/such.gguf","inputs":["a"]}"#);
    assert_eq!(
        lines,
        [
            r#"{"type":"error","message":"Failed to load model: unable to load model at \"/no/such.gguf\""}"#
        ]
    );
}
