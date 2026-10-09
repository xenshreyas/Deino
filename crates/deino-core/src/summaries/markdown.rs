//! Cleaning model output into displayable Markdown.
//!
//! Local models sometimes wrap reasoning in `<think>`/`<thinking>` envelopes
//! or the whole answer in a code fence. [`clean_markdown`] removes closed
//! envelopes anywhere and one outer fence; an unterminated marker is left in
//! place for [`require_visible`] to reject, so private reasoning is never
//! shown or stored by accident. Ported from the legacy `summary/processor.rs`.

use std::sync::LazyLock;

use regex::Regex;

use super::SummaryError;

/// A closed reasoning envelope, attributes allowed: `<think …>…</think>`.
static ENVELOPE: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"(?is)<think(?:ing)?(?:\s+[^>]*)?>.*?</think(?:ing)?\s*>").ok());

/// Any opening or closing reasoning marker.
static MARKER: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"(?is)</?think(?:ing)?(?:\s+[^>]*)?>").ok());

/// Outer fences a model may wrap its answer in.
const FENCE_OPENINGS: [&str; 4] = ["```markdown\n", "```\n", "```markdown\r\n", "```\r\n"];
const FENCE_CLOSE: &str = "```";

/// Model output with reasoning envelopes and an outer fence removed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CleanedMarkdown {
    /// The visible Markdown.
    pub markdown: String,
    /// Whether a reasoning envelope was removed.
    pub reasoning_stripped: bool,
}

/// Cleans `raw` model output (see the module docs).
pub fn clean_markdown(raw: &str) -> CleanedMarkdown {
    let visible = ENVELOPE
        .as_ref()
        .map_or_else(|| raw.into(), |envelope| envelope.replace_all(raw, ""));
    let reasoning_stripped = visible.as_ref() != raw;
    let trimmed = visible.trim();
    let markdown = FENCE_OPENINGS
        .iter()
        .find_map(|opening| {
            trimmed
                .strip_prefix(opening)
                .and_then(|inner| inner.strip_suffix(FENCE_CLOSE))
                .map(str::trim)
        })
        .unwrap_or(trimmed)
        .to_owned();
    CleanedMarkdown {
        markdown,
        reasoning_stripped,
    }
}

/// Whether `markdown` still holds a reasoning marker (an unterminated one).
pub fn contains_reasoning_marker(markdown: &str) -> bool {
    MARKER
        .as_ref()
        .is_some_and(|marker| marker.is_match(markdown))
}

/// Rejects output with an unterminated reasoning marker or nothing visible.
pub fn require_visible(stage: &'static str, cleaned: &CleanedMarkdown) -> Result<(), SummaryError> {
    if contains_reasoning_marker(&cleaned.markdown) {
        Err(SummaryError::ReasoningMarker { stage })
    } else if cleaned.markdown.is_empty() {
        Err(SummaryError::NoVisibleContent { stage })
    } else {
        Ok(())
    }
}

/// The text of the first `# ` heading, which the legacy summaries use as the
/// meeting's title.
pub fn first_heading(markdown: &str) -> Option<String> {
    markdown
        .lines()
        .find_map(|line| line.strip_prefix("# "))
        .map(|title| title.trim().to_owned())
        .filter(|title| !title.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closed_envelopes_are_removed_everywhere() {
        let cleaned = clean_markdown(
            "Intro\n<think>private</think>\n# Meeting\n<thinking class=\"x\">also</thinking>\nTail",
        );

        assert!(cleaned.reasoning_stripped);
        assert!(!cleaned.markdown.contains("private"));
        assert!(!cleaned.markdown.contains("also"));
        assert!(cleaned.markdown.contains("# Meeting") && cleaned.markdown.ends_with("Tail"));
    }

    #[test]
    fn an_outer_fence_is_removed_without_touching_inner_markdown() {
        let inner = "# Heading\r\n\r\n```rust\r\nlet x = 1;\r\n```";

        assert_eq!(
            clean_markdown(&format!("```\r\n{inner}\r\n```")).markdown,
            inner
        );
        assert_eq!(
            clean_markdown("```\n<think>p</think>\nvisible\n```").markdown,
            "visible"
        );
        assert_eq!(
            clean_markdown("<thinker>Visible</thinker>").markdown,
            "<thinker>Visible</thinker>"
        );
    }

    #[test]
    fn unterminated_markers_and_empty_output_are_rejected() {
        for raw in ["Visible\n<think>private", "Visible\n</thinking>"] {
            assert!(matches!(
                require_visible("Final summary", &clean_markdown(raw)),
                Err(SummaryError::ReasoningMarker { .. })
            ));
        }
        for raw in ["<think>private</think>", "```\n```", "```markdown\r\n```"] {
            assert!(matches!(
                require_visible("Final summary", &clean_markdown(raw)),
                Err(SummaryError::NoVisibleContent { .. })
            ));
        }
    }

    #[test]
    fn the_first_h1_is_the_title() {
        assert_eq!(
            first_heading("Intro\n# Brightkite pilot\n## Next"),
            Some("Brightkite pilot".into())
        );
        assert_eq!(first_heading("## Only a subheading"), None);
    }
}
