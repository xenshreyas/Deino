//! Ollama's native API (`/api/tags`, `/api/show`, `/api/pull`): wire types,
//! the context-window lookup, and newline-delimited JSON streams.

use std::collections::{BTreeMap, VecDeque};
use std::mem;

use futures_util::StreamExt;
use futures_util::stream::{self, BoxStream};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{BodyStream, HttpError};

/// Context sizes by model family, when the server does not report one (the
/// legacy table, from each family's published defaults).
const FAMILY_CONTEXT: [(&str, u32); 8] = [
    ("llama", 4096),
    ("mistral", 8192),
    ("phi", 2048),
    ("qwen", 8192),
    ("gemma", 8192),
    ("codellama", 16384),
    ("deepseek", 16384),
    ("neural-chat", 4096),
];

/// `GET /api/tags`.
#[derive(Debug, Deserialize)]
pub(super) struct Tags {
    pub models: Vec<Tag>,
}

#[derive(Debug, Deserialize)]
pub(super) struct Tag {
    pub name: String,
    pub modified_at: Option<String>,
    pub size: Option<u64>,
}

/// `POST /api/show` and `POST /api/pull` / `DELETE /api/delete` bodies.
#[derive(Debug, Serialize)]
pub(super) struct ModelRequest<'a> {
    pub model: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verbose: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
}

/// `POST /api/show` (verbose).
#[derive(Debug, Default, Deserialize)]
pub(super) struct Show {
    #[serde(default)]
    pub modelfile: String,
    #[serde(default)]
    pub details: Details,
    #[serde(default)]
    pub model_info: BTreeMap<String, Value>,
}

#[derive(Debug, Default, Deserialize)]
pub(super) struct Details {
    #[serde(default)]
    pub family: String,
}

/// One line of a `POST /api/pull` stream.
#[derive(Debug, Deserialize)]
pub(super) struct PullLine {
    pub status: Option<String>,
    pub completed: Option<u64>,
    pub total: Option<u64>,
    pub error: Option<String>,
}

impl Show {
    /// The context window: the model's reported length, else a `num_ctx`
    /// parameter in its modelfile, else its family's default.
    pub(super) fn context_window(&self, model: &str) -> Option<u32> {
        self.reported_length()
            .or_else(|| num_ctx(&self.modelfile))
            .or_else(|| family_default(&self.details.family))
            .or_else(|| family_default(model))
    }

    fn reported_length(&self) -> Option<u32> {
        self.model_info
            .iter()
            .find(|(key, _)| key.ends_with("context_length"))
            .and_then(|(_, value)| value.as_u64())
            .and_then(|length| u32::try_from(length).ok())
    }
}

fn num_ctx(modelfile: &str) -> Option<u32> {
    modelfile.lines().find_map(|line| {
        let mut words = line.split_whitespace();
        match (words.next(), words.next(), words.next()) {
            (Some("PARAMETER"), Some("num_ctx"), Some(value)) => value.parse().ok(),
            _ => None,
        }
    })
}

/// The default context of the family a model name belongs to.
pub(super) fn family_default(name: &str) -> Option<u32> {
    let name = name.to_ascii_lowercase();
    FAMILY_CONTEXT
        .iter()
        .find(|(family, _)| name.contains(family))
        .map(|(_, size)| *size)
}

/// The lines of a newline-delimited body, without their line ends; blank
/// lines are skipped. A transport error ends the stream.
pub(super) fn lines(body: BodyStream) -> BoxStream<'static, Result<String, HttpError>> {
    let state = (body, Vec::new(), VecDeque::new(), false);
    stream::unfold(
        state,
        |(mut body, mut partial, mut ready, mut done)| async move {
            loop {
                if let Some(line) = ready.pop_front() {
                    return Some((Ok(line), (body, partial, ready, done)));
                }
                if done {
                    return None;
                }
                match body.next().await {
                    Some(Ok(chunk)) => split_lines(&mut partial, &chunk, &mut ready),
                    Some(Err(error)) => return Some((Err(error), (body, partial, ready, true))),
                    None => {
                        done = true;
                        let rest = String::from_utf8_lossy(&mem::take(&mut partial))
                            .trim()
                            .to_owned();
                        ready.extend((!rest.is_empty()).then_some(rest));
                    }
                }
            }
        },
    )
    .boxed()
}

fn split_lines(partial: &mut Vec<u8>, chunk: &[u8], ready: &mut VecDeque<String>) {
    for &byte in chunk {
        if byte != b'\n' {
            partial.push(byte);
            continue;
        }
        let line = String::from_utf8_lossy(&mem::take(partial))
            .trim()
            .to_owned();
        if !line.is_empty() {
            ready.push_back(line);
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn the_reported_context_length_wins() {
        let show: Show = serde_json::from_value(json!({
            "modelfile": "PARAMETER num_ctx 4096",
            "details": { "family": "gemma3" },
            "model_info": { "gemma3.context_length": 131_072 },
        }))
        .unwrap();

        assert_eq!(show.context_window("gemma3:4b"), Some(131_072));
    }

    #[test]
    fn num_ctx_then_the_family_default_fill_in() {
        let with_num_ctx = Show {
            modelfile: "FROM x\nPARAMETER   num_ctx   16384\n".into(),
            ..Show::default()
        };

        assert_eq!(with_num_ctx.context_window("custom"), Some(16_384));
        assert_eq!(Show::default().context_window("mistral:7b"), Some(8192));
        assert_eq!(Show::default().context_window("mystery"), None);
    }

    #[tokio::test]
    async fn lines_split_across_chunks_and_skip_blanks() {
        let body: BodyStream = stream::iter([
            Ok(b"{\"a\":1}\n{\"b\"".to_vec()),
            Ok(b":2}\n\n{\"c\":3}".to_vec()),
        ])
        .boxed();

        let read: Vec<String> = lines(body).map(Result::unwrap).collect().await;

        assert_eq!(read, [r#"{"a":1}"#, r#"{"b":2}"#, r#"{"c":3}"#]);
    }
}
