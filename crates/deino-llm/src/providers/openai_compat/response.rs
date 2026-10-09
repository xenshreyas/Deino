//! OpenAI Chat Completions response types: streamed chunks and the model list.

use serde::Deserialize;
use serde_json::Value;

/// One streamed chunk.
#[derive(Debug, Deserialize)]
pub(super) struct Chunk {
    #[serde(default)]
    pub choices: Vec<Choice>,
    pub usage: Option<WireUsage>,
    pub error: Option<Value>,
}

#[derive(Debug, Deserialize)]
pub(super) struct Choice {
    #[serde(default)]
    pub delta: Delta,
    pub finish_reason: Option<String>,
}

/// A choice's increment. Reasoning fields (`reasoning`, `reasoning_content`)
/// are not read: reasoning is never shown or stored.
#[derive(Debug, Default, Deserialize)]
pub(super) struct Delta {
    pub content: Option<String>,
    #[serde(default)]
    pub tool_calls: Vec<ToolCallDelta>,
}

#[derive(Debug, Deserialize)]
pub(super) struct ToolCallDelta {
    #[serde(default)]
    pub index: usize,
    pub id: Option<String>,
    pub function: Option<FunctionDelta>,
}

#[derive(Debug, Deserialize)]
pub(super) struct FunctionDelta {
    pub name: Option<String>,
    pub arguments: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize)]
pub(super) struct WireUsage {
    #[serde(default)]
    pub prompt_tokens: u32,
    #[serde(default)]
    pub completion_tokens: u32,
}

/// `GET {base}/models`.
#[derive(Debug, Deserialize)]
pub(super) struct ModelList {
    pub data: Vec<ModelEntry>,
}

#[derive(Debug, Deserialize)]
pub(super) struct ModelEntry {
    pub id: String,
    pub name: Option<String>,
    pub context_length: Option<u32>,
    pub top_provider: Option<TopProvider>,
}

#[derive(Debug, Deserialize)]
pub(super) struct TopProvider {
    pub context_length: Option<u32>,
}
