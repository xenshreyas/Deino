//! Sampling: sanitising a request's options and building the sampler chain.
//!
//! [`SamplingConfig::from_options`] replaces absent or invalid values with
//! safe defaults. [`plan`] decides the order of the llama.cpp samplers as
//! plain data, so tests can check it without a model, and [`build`] turns
//! the plan into a `LlamaSampler` chain. A grammar sampler always comes
//! first, so every later stage only sees tokens the grammar allows.

use llama_cpp_2::sampling::LlamaSampler;

use crate::protocol::SamplingOptions;

/// Temperature when the request does not set one.
const DEFAULT_TEMPERATURE: f32 = 1.0;
/// `top_k` when the request does not set one.
const DEFAULT_TOP_K: i32 = 64;
/// `top_p` when the request does not set one.
const DEFAULT_TOP_P: f32 = 0.95;
/// The repetition penalty that changes nothing.
const NEUTRAL_REPEAT_PENALTY: f32 = 1.0;
/// `min_keep` for the `top_p` sampler: always keep at least one token.
const TOP_P_MIN_KEEP: usize = 1;

/// Sampling parameters after sanitising; every value is safe to hand to llama.cpp.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SamplingConfig {
    /// Softmax temperature, at least 0; 0 means greedy decoding.
    pub temperature: f32,
    /// At least 1.
    pub top_k: i32,
    /// In `(0, 1]`.
    pub top_p: f32,
    /// At least 0.
    pub presence_penalty: f32,
    /// At least 0.
    pub frequency_penalty: f32,
    /// Greater than 0.
    pub repeat_penalty: f32,
    /// At least 0.
    pub penalty_last_n: i32,
}

impl SamplingConfig {
    /// Fills in defaults and replaces values llama.cpp cannot use.
    pub fn from_options(options: &SamplingOptions) -> Self {
        let temperature = options.temperature.unwrap_or(DEFAULT_TEMPERATURE);
        let top_p = options.top_p.unwrap_or(DEFAULT_TOP_P);
        let repeat_penalty = options.repeat_penalty.unwrap_or(NEUTRAL_REPEAT_PENALTY);
        Self {
            temperature: finite_or(temperature, |_| true, 0.0).max(0.0),
            top_k: options.top_k.unwrap_or(DEFAULT_TOP_K).max(1),
            top_p: finite_or(top_p, |p| p > 0.0 && p <= 1.0, 1.0),
            presence_penalty: non_negative(options.presence_penalty),
            frequency_penalty: non_negative(options.frequency_penalty),
            repeat_penalty: finite_or(repeat_penalty, |r| r > 0.0, NEUTRAL_REPEAT_PENALTY),
            penalty_last_n: options.penalty_last_n.unwrap_or(0).max(0),
        }
    }

    /// Whether any penalty is active over a non-empty window.
    pub fn uses_penalties(&self) -> bool {
        self.penalty_last_n > 0
            && (self.presence_penalty > 0.0
                || self.frequency_penalty > 0.0
                || (self.repeat_penalty - NEUTRAL_REPEAT_PENALTY).abs() > f32::EPSILON)
    }

    /// Whether decoding is greedy (temperature 0).
    pub fn is_greedy(&self) -> bool {
        self.temperature <= 0.0
    }
}

/// `value` when it is finite and `valid`, otherwise `fallback`.
fn finite_or(value: f32, valid: impl Fn(f32) -> bool, fallback: f32) -> f32 {
    if value.is_finite() && valid(value) {
        value
    } else {
        fallback
    }
}

/// A penalty that defaults to 0 and is never negative or infinite.
fn non_negative(value: Option<f32>) -> f32 {
    finite_or(value.unwrap_or(0.0), |_| true, 0.0).max(0.0)
}

/// One sampler in the chain, in the order [`plan`] lists them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    /// Masks tokens the grammar does not allow.
    Grammar,
    /// Presence, frequency and repetition penalties.
    Penalties,
    /// Keeps the `top_k` most likely tokens.
    TopK,
    /// Keeps the smallest set whose probability reaches `top_p`.
    TopP,
    /// Scales logits by the temperature.
    Temperature,
    /// Draws a token at random from what is left.
    Distribution,
    /// Picks the most likely token.
    Greedy,
}

/// The sampler stages for `config`, with a grammar first when `constrained`.
pub fn plan(config: &SamplingConfig, constrained: bool) -> Vec<Stage> {
    let mut stages = Vec::with_capacity(6);
    if constrained {
        stages.push(Stage::Grammar);
    }
    if config.uses_penalties() {
        stages.push(Stage::Penalties);
    }
    if config.is_greedy() {
        stages.push(Stage::Greedy);
    } else {
        stages.extend([
            Stage::TopK,
            Stage::TopP,
            Stage::Temperature,
            Stage::Distribution,
        ]);
    }
    stages
}

/// Builds the sampler chain for `config`, putting `grammar` first if given.
pub fn build(config: &SamplingConfig, grammar: Option<LlamaSampler>, seed: u32) -> LlamaSampler {
    let mut grammar = grammar;
    let stages = plan(config, grammar.is_some());
    let samplers = stages.into_iter().filter_map(|stage| match stage {
        Stage::Grammar => grammar.take(),
        other => Some(sampler(config, other, seed)),
    });
    LlamaSampler::chain_simple(samplers)
}

/// The llama.cpp sampler for one stage. `build` supplies the grammar sampler
/// itself, so `Stage::Grammar` never reaches the greedy fallback here.
fn sampler(config: &SamplingConfig, stage: Stage, seed: u32) -> LlamaSampler {
    match stage {
        Stage::Penalties => LlamaSampler::penalties(
            config.penalty_last_n,
            config.repeat_penalty,
            config.frequency_penalty,
            config.presence_penalty,
        ),
        Stage::TopK => LlamaSampler::top_k(config.top_k),
        Stage::TopP => LlamaSampler::top_p(config.top_p, TOP_P_MIN_KEEP),
        Stage::Temperature => LlamaSampler::temp(config.temperature),
        Stage::Distribution => LlamaSampler::dist(seed),
        Stage::Greedy | Stage::Grammar => LlamaSampler::greedy(),
    }
}

#[cfg(test)]
#[allow(
    clippy::float_cmp,
    reason = "sanitised values are copied from the input or a constant, never computed"
)]
mod tests {
    use super::*;
    use crate::protocol::Request;

    fn sampling_of(line: &str) -> SamplingConfig {
        let Request::Generate(request) = serde_json::from_str(line).unwrap() else {
            panic!("expected a generate request");
        };
        SamplingConfig::from_options(&request.sampling)
    }

    #[test]
    fn generate_request_defaults_penalties_when_omitted() {
        let sampling = sampling_of(
            r#"{"type":"generate","prompt":"summarize","temperature":0.5,"top_k":20,"top_p":0.8}"#,
        );
        assert_eq!(sampling.presence_penalty, 0.0);
        assert_eq!(sampling.frequency_penalty, 0.0);
        assert_eq!(sampling.repeat_penalty, 1.0);
        assert_eq!(sampling.penalty_last_n, 0);
        assert!(!sampling.uses_penalties());
    }

    #[test]
    fn generate_request_deserializes_qwen_penalties() {
        let sampling = sampling_of(
            r#"{"type":"generate","prompt":"summarize","temperature":0.5,"top_k":20,"top_p":0.8,"presence_penalty":0.3,"frequency_penalty":0.0,"repeat_penalty":1.05,"penalty_last_n":256}"#,
        );
        assert_eq!(sampling.temperature, 0.5);
        assert_eq!(sampling.top_k, 20);
        assert_eq!(sampling.top_p, 0.8);
        assert_eq!(sampling.presence_penalty, 0.3);
        assert_eq!(sampling.frequency_penalty, 0.0);
        assert_eq!(sampling.repeat_penalty, 1.05);
        assert_eq!(sampling.penalty_last_n, 256);
        assert!(sampling.uses_penalties());
    }

    #[test]
    fn defaults_when_nothing_is_set() {
        let sampling = SamplingConfig::from_options(&SamplingOptions::default());
        assert_eq!(sampling.temperature, 1.0);
        assert_eq!(sampling.top_k, 64);
        assert_eq!(sampling.top_p, 0.95);
    }

    #[test]
    fn replaces_invalid_values() {
        let sampling = SamplingConfig::from_options(&SamplingOptions {
            temperature: Some(f32::NAN),
            top_k: Some(-3),
            top_p: Some(1.5),
            presence_penalty: Some(-1.0),
            frequency_penalty: Some(f32::INFINITY),
            repeat_penalty: Some(0.0),
            penalty_last_n: Some(-5),
        });
        let expected = SamplingConfig {
            temperature: 0.0,
            top_k: 1,
            top_p: 1.0,
            presence_penalty: 0.0,
            frequency_penalty: 0.0,
            repeat_penalty: 1.0,
            penalty_last_n: 0,
        };
        assert_eq!(sampling, expected);
    }

    #[test]
    fn plans_the_legacy_chains() {
        let mut config = SamplingConfig::from_options(&SamplingOptions::default());
        let random = [
            Stage::TopK,
            Stage::TopP,
            Stage::Temperature,
            Stage::Distribution,
        ];
        assert_eq!(plan(&config, false), random);
        config.penalty_last_n = 64;
        config.presence_penalty = 0.5;
        assert_eq!(plan(&config, false)[0], Stage::Penalties);
        config.temperature = 0.0;
        assert_eq!(plan(&config, false), [Stage::Penalties, Stage::Greedy]);
    }

    #[test]
    fn the_grammar_always_comes_first() {
        let mut config = SamplingConfig::from_options(&SamplingOptions::default());
        config.penalty_last_n = 64;
        config.repeat_penalty = 1.1;
        let stages = plan(&config, true);
        assert_eq!(stages[..2], [Stage::Grammar, Stage::Penalties]);
        config.temperature = 0.0;
        assert_eq!(
            plan(&config, true),
            [Stage::Grammar, Stage::Penalties, Stage::Greedy]
        );
    }
}
