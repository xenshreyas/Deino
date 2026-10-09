//! [`SystemContext`]: standing context the gateway puts in front of every
//! system prompt (NOT-05).
//!
//! The app wires the user's profile here (`deino_core::profile::ProfileContext`):
//! who they are, their role, focus areas, writing style and glossary. Every
//! text route gets it, so notes, chat, titles and tags are all written for
//! this user without each caller remembering to ask.

use async_trait::async_trait;

use crate::Prompt;

/// Gives every LLM call the user's standing context.
#[async_trait]
pub trait SystemContext: Send + Sync {
    /// The snippet to put at the top of the system prompt, or `None` when
    /// there is nothing to say.
    async fn system_context(&self) -> Option<String>;
}

/// `prompt` with `context` in front of its system prompt (or as its system
/// prompt when it has none).
pub(crate) fn with_context(mut prompt: Prompt, context: Option<String>) -> Prompt {
    prompt.system = match (context, prompt.system.take()) {
        (Some(context), Some(system)) => Some(format!("{context}\n\n{system}")),
        (context, system) => context.or(system),
    };
    prompt
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_context_comes_first_and_either_side_may_be_missing() {
        let both = with_context(
            Prompt::new("hi").system("Title it."),
            Some("About me".into()),
        );
        let only_context = with_context(Prompt::new("hi"), Some("About me".into()));
        let only_system = with_context(Prompt::new("hi").system("Title it."), None);

        assert_eq!(both.system.as_deref(), Some("About me\n\nTitle it."));
        assert_eq!(only_context.system.as_deref(), Some("About me"));
        assert_eq!(only_system.system.as_deref(), Some("Title it."));
    }
}
