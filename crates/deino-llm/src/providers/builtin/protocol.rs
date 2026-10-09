//! The app's half of the llama-helper JSON-lines protocol
//! (`llama-helper/src/protocol.rs` is the other half; tests on both sides
//! pin the same JSON).
//!
//! Requests the app writes: `stream`, `grammar` (a stream constrained by a
//! JSON schema) and `embed`. Lines it reads back: [`HelperEvent`] for a
//! stream, [`EmbedReply`] for an embedding.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// One request line.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HelperRequest {
    /// Stream a completion as `token` lines.
    Stream(HelperCompletion),
    /// Like `stream`, constrained to `json_schema`.
    Grammar {
        /// The completion fields.
        #[serde(flatten)]
        completion: HelperCompletion,
        /// The schema the answer must match.
        json_schema: Value,
    },
    /// Embed texts with an embedding model.
    Embed {
        /// The GGUF embedding model.
        model_path: String,
        /// The texts, one vector each.
        inputs: Vec<String>,
    },
}

/// A completion's fields.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct HelperCompletion {
    /// The prompt, the model's chat template already applied.
    pub prompt: String,
    /// The most tokens to generate.
    pub max_tokens: u32,
    /// The context window to load the model with.
    pub context_size: u32,
    /// The GGUF chat model.
    pub model_path: String,
    /// The model's sampling parameters.
    #[serde(flatten)]
    pub sampling: HelperSampling,
    /// Texts that end generation.
    pub stop_tokens: Vec<String>,
}

/// Sampling parameters, as each built-in model defines them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize)]
pub struct HelperSampling {
    /// Softmax temperature.
    pub temperature: f32,
    /// Keep the `top_k` most likely tokens.
    pub top_k: i32,
    /// Nucleus sampling threshold.
    pub top_p: f32,
    /// Flat penalty for tokens already seen.
    pub presence_penalty: f32,
    /// Penalty growing with how often a token was seen.
    pub frequency_penalty: f32,
    /// Multiplicative repetition penalty.
    pub repeat_penalty: f32,
    /// How many recent tokens the penalties look at.
    pub penalty_last_n: i32,
}

/// The answer to `embed`.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EmbedReply {
    /// One L2-normalised vector per input, in order.
    Embeddings {
        /// The vectors.
        vectors: Vec<Vec<f32>>,
    },
    /// The embedding failed.
    Error {
        /// What went wrong.
        message: String,
    },
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn a_grammar_request_flattens_the_completion_and_adds_the_schema() {
        let request = HelperRequest::Grammar {
            completion: HelperCompletion {
                prompt: "p".into(),
                max_tokens: 64,
                context_size: 32_768,
                model_path: "/m.gguf".into(),
                sampling: HelperSampling {
                    temperature: 0.5,
                    top_k: 20,
                    top_p: 0.8,
                    presence_penalty: 0.3,
                    frequency_penalty: 0.0,
                    repeat_penalty: 1.05,
                    penalty_last_n: 256,
                },
                stop_tokens: vec!["<|im_end|>".into()],
            },
            json_schema: json!({ "type": "object" }),
        };

        let line = serde_json::to_value(&request).unwrap();

        assert_eq!(line["type"], "grammar");
        assert_eq!(line["prompt"], "p");
        assert_eq!(line["top_k"], 20);
        assert_eq!(line["penalty_last_n"], 256);
        assert_eq!(line["stop_tokens"], json!(["<|im_end|>"]));
        assert_eq!(line["json_schema"], json!({ "type": "object" }));
    }

    #[test]
    fn stream_and_embed_requests_are_tagged() {
        let stream =
            serde_json::to_value(HelperRequest::Stream(HelperCompletion::default())).unwrap();
        let embed = serde_json::to_value(HelperRequest::Embed {
            model_path: "/e.gguf".into(),
            inputs: vec!["a".into()],
        })
        .unwrap();

        assert_eq!(stream["type"], "stream");
        assert_eq!(
            embed,
            json!({ "type": "embed", "model_path": "/e.gguf", "inputs": ["a"] })
        );
    }

    #[test]
    fn embed_replies_deserialize() {
        let reply: EmbedReply = serde_json::from_str(
            r#"{"type":"embeddings","vectors":[[0.6,0.8]],"prompt_tokens":3}"#,
        )
        .unwrap();

        assert_eq!(
            reply,
            EmbedReply::Embeddings {
                vectors: vec![vec![0.6, 0.8]]
            }
        );
    }
}
