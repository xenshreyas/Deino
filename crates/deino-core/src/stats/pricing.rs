//! Estimated LLM cost: tokens times list price.
//!
//! `llm_calls` stores tokens, never money, because the provider bills and
//! its prices change; the Stats page labels every amount as an estimate.
//! Prices are Anthropic's first-party list prices per million tokens (as of
//! 2026-10), without caching or batch discounts. Local providers (Ollama,
//! the built-in model) run on the Mac and cost nothing. Any other model is
//! unpriced and left out of the estimate rather than guessed.

use deino_llm::ProviderId;

/// Tokens a list price is quoted for.
const TOKENS_PER_PRICE: u128 = 1_000_000;

/// A model's list price, in millionths of a dollar per million tokens.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Price {
    /// Per million prompt tokens.
    pub input_micros: u64,
    /// Per million answer tokens.
    pub output_micros: u64,
}

impl Price {
    /// A price from whole cents per million tokens.
    const fn cents(input: u64, output: u64) -> Self {
        Self {
            input_micros: input * 10_000,
            output_micros: output * 10_000,
        }
    }

    /// Nothing: a model that runs on the Mac.
    const FREE: Self = Self::cents(0, 0);

    /// The estimated cost of `input` and `output` tokens, in millionths of
    /// a dollar, rounded to the nearest one.
    pub fn cost_micros(self, input: u64, output: u64) -> u64 {
        let total = u128::from(input) * u128::from(self.input_micros)
            + u128::from(output) * u128::from(self.output_micros);
        let micros = (total + TOKENS_PER_PRICE / 2) / TOKENS_PER_PRICE;
        u64::try_from(micros).unwrap_or(u64::MAX)
    }
}

/// Anthropic's models by ID, with their list prices.
const ANTHROPIC: [(&str, Price); 13] = [
    ("claude-fable-5-1", Price::cents(1_000, 5_000)),
    ("claude-fable-5", Price::cents(1_000, 5_000)),
    ("claude-opus-5-5", Price::cents(400, 2_000)),
    ("claude-opus-5", Price::cents(500, 2_500)),
    ("claude-opus-4-8", Price::cents(500, 2_500)),
    ("claude-opus-4-7", Price::cents(500, 2_500)),
    ("claude-opus-4-6", Price::cents(500, 2_500)),
    ("claude-sonnet-5-5", Price::cents(200, 1_000)),
    ("claude-sonnet-5", Price::cents(200, 1_000)),
    ("claude-sonnet-4-6", Price::cents(300, 1_500)),
    ("claude-sonnet-4-5", Price::cents(300, 1_500)),
    ("claude-haiku-5-5", Price::cents(10, 50)),
    ("claude-haiku-4-5", Price::cents(100, 500)),
];

/// The list price of `model` at `provider`, or `None` when it is not known.
pub fn price_of(provider: ProviderId, model: &str) -> Option<Price> {
    match provider {
        ProviderId::Ollama | ProviderId::BuiltIn => Some(Price::FREE),
        ProviderId::Anthropic => ANTHROPIC
            .iter()
            .find(|(id, _)| *id == model)
            .map(|(_, price)| *price),
        ProviderId::OpenAi
        | ProviderId::Groq
        | ProviderId::OpenRouter
        | ProviderId::CustomOpenAi => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_million_tokens_each_way_cost_the_list_price() {
        let sonnet = price_of(ProviderId::Anthropic, "claude-sonnet-5-5").unwrap();

        // $2 + $10.
        assert_eq!(sonnet.cost_micros(1_000_000, 1_000_000), 12_000_000);
    }

    #[test]
    fn small_calls_round_to_the_nearest_millionth() {
        let haiku = price_of(ProviderId::Anthropic, "claude-haiku-5-5").unwrap();

        // 1,234 × $0.10/M + 567 × $0.50/M = 123.4 + 283.5 = 406.9 µ$.
        assert_eq!(haiku.cost_micros(1_234, 567), 407);
    }

    #[test]
    fn local_models_are_free_and_unknown_ones_are_unpriced() {
        assert_eq!(
            price_of(ProviderId::BuiltIn, "qwen3.5:4b").map(|p| p.cost_micros(9, 9)),
            Some(0)
        );
        assert_eq!(price_of(ProviderId::Ollama, "gemma3:1b"), Some(Price::FREE));
        assert_eq!(price_of(ProviderId::OpenAi, "gpt-5"), None);
        assert_eq!(price_of(ProviderId::Anthropic, "claude-unknown"), None);
    }
}
