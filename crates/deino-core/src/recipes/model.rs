//! [`Recipe`]: a saved prompt, and the [`RecipeDraft`] the user writes.

use chrono::{DateTime, Utc};
use deino_domain::RecipeId;
use serde::{Deserialize, Serialize};

use super::RecipeError;

/// The longest name, in characters.
pub const MAX_NAME_CHARS: usize = 80;
/// The longest description, in characters.
pub const MAX_DESCRIPTION_CHARS: usize = 280;
/// The longest prompt, in characters.
pub const MAX_PROMPT_CHARS: usize = 8_000;

/// What a recipe runs on.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type, sqlx::Type,
)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum RecipeKind {
    /// One meeting: the "Ask about this meeting" bar and Preview.
    #[default]
    Single,
    /// Several meetings (Home, folders, people); WIP until AI-02 (D13).
    Multi,
}

/// A saved prompt.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Recipe {
    /// The recipe.
    pub id: RecipeId,
    /// Its `/shortcut`: lowercase words joined by hyphens, unique.
    pub slug: String,
    /// What the user calls it.
    pub name: String,
    /// One line on what it does.
    pub description: String,
    /// What it asks the model.
    pub prompt: String,
    /// What it runs on.
    pub kind: RecipeKind,
    /// True for the app's library; those are read-only.
    pub built_in: bool,
    /// When it was created.
    pub created_at: DateTime<Utc>,
    /// When it was last changed.
    pub updated_at: DateTime<Utc>,
}

impl Recipe {
    /// Whether it can run on one meeting today.
    pub fn is_runnable(&self) -> bool {
        self.kind == RecipeKind::Single
    }
}

/// What the user writes: a new recipe, an edit, or an import.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RecipeDraft {
    /// The name; its slug is derived from it.
    pub name: String,
    /// One line on what it does.
    pub description: String,
    /// What it asks the model.
    pub prompt: String,
}

impl RecipeDraft {
    /// The draft trimmed, or why it cannot be saved.
    pub fn validated(self) -> Result<Self, RecipeError> {
        let name = self.name.trim().to_owned();
        let description = self.description.trim().to_owned();
        let prompt = self.prompt.trim().to_owned();
        check(&name, "name", MAX_NAME_CHARS)?;
        check(&prompt, "prompt", MAX_PROMPT_CHARS)?;
        if description.chars().count() > MAX_DESCRIPTION_CHARS {
            return Err(RecipeError::invalid("description", "is too long"));
        }
        Ok(Self {
            name,
            description,
            prompt,
        })
    }
}

fn check(value: &str, field: &'static str, limit: usize) -> Result<(), RecipeError> {
    if value.is_empty() {
        return Err(RecipeError::invalid(field, "is empty"));
    }
    if value.chars().count() > limit {
        return Err(RecipeError::invalid(field, "is too long"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draft(name: &str, prompt: &str) -> RecipeDraft {
        RecipeDraft {
            name: name.into(),
            description: "  ".into(),
            prompt: prompt.into(),
        }
    }

    #[test]
    fn a_draft_is_trimmed() {
        let draft = draft("  Coach me ", " Coach me. ").validated().unwrap();

        assert_eq!(
            (draft.name.as_str(), draft.description.as_str()),
            ("Coach me", "")
        );
        assert_eq!(draft.prompt, "Coach me.");
    }

    #[test]
    fn empty_and_overlong_fields_are_refused() {
        assert!(draft(" ", "x").validated().is_err());
        assert!(draft("x", "").validated().is_err());
        let long = "x".repeat(MAX_PROMPT_CHARS + 1);
        assert!(matches!(
            draft("x", &long).validated(),
            Err(RecipeError::Invalid {
                field: "prompt",
                ..
            })
        ));
    }
}
