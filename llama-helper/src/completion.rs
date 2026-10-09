//! One completion request, from the wire fields to generated text.
//!
//! [`run`] loads the chat model when the request names one, builds a grammar
//! sampler for `grammar` requests, applies the defaults and generates into a
//! sink. [`CompletionError`] keeps the failing phase, so `generate` can
//! report it in its legacy wording and `stream` with the full error chain.

use std::path::PathBuf;

use crate::generate::TokenSink;
use crate::grammar;
use crate::model::{ModelStore, SlotKind};
use crate::protocol::{CompletionRequest, DEFAULT_CONTEXT_SIZE, DEFAULT_MAX_TOKENS};
use crate::sampling::SamplingConfig;
use crate::source::{self, GenerationParams, Outcome};

/// A completion to run, constrained by a JSON schema for `grammar` requests.
#[derive(Debug)]
pub struct CompletionJob {
    /// The request fields.
    pub request: CompletionRequest,
    /// The JSON schema the output must match, if any.
    pub schema: Option<serde_json::Value>,
}

/// Why a completion failed, by phase.
#[derive(Debug)]
pub enum CompletionError {
    /// The model named by the request could not be loaded.
    Load(anyhow::Error),
    /// The JSON schema or its grammar was rejected.
    Grammar(anyhow::Error),
    /// No model was loaded, or generation itself failed.
    Generate(anyhow::Error),
}

impl CompletionError {
    /// The `error` of a legacy `response` line: the outermost cause only.
    pub fn legacy_message(&self) -> String {
        match self {
            Self::Load(error) => format!("Failed to load model: {error}"),
            Self::Grammar(error) => format!("{error}"),
            Self::Generate(error) => format!("Generation failed: {error}"),
        }
    }

    /// The `message` of an `error` line: the whole cause chain.
    pub fn message(&self) -> String {
        match self {
            Self::Load(error) => format!("Failed to load model: {error:#}"),
            Self::Grammar(error) => format!("{error:#}"),
            Self::Generate(error) => format!("Generation failed: {error:#}"),
        }
    }
}

/// Runs `job` with the chat model, emitting the text into `sink`.
pub fn run(
    store: &mut ModelStore,
    job: CompletionJob,
    sink: &mut impl TokenSink,
) -> Result<Outcome, CompletionError> {
    let CompletionJob { request, schema } = job;
    if let Some(path) = &request.model_path {
        let context_size = request.context_size.unwrap_or(DEFAULT_CONTEXT_SIZE);
        store
            .load(SlotKind::Chat, PathBuf::from(path), context_size)
            .map_err(CompletionError::Load)?;
    }
    let engine = store
        .engine(SlotKind::Chat)
        .map_err(CompletionError::Generate)?;
    let grammar = schema
        .map(|schema| grammar::grammar_sampler(&engine.loaded.model, &schema))
        .transpose()
        .map_err(CompletionError::Grammar)?;
    let stop_tokens = request.stop_tokens.unwrap_or_default();
    let params = GenerationParams {
        prompt: &request.prompt,
        max_tokens: request.max_tokens.unwrap_or(DEFAULT_MAX_TOKENS),
        sampling: SamplingConfig::from_options(&request.sampling),
        stop_tokens: &stop_tokens,
        grammar,
    };
    source::generate(engine, params, sink).map_err(CompletionError::Generate)
}
