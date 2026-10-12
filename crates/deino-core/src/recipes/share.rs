//! Sharing recipes: a JSON file of several (`"format": "deino.recipes"`),
//! and the share string `deino://recipe?data=<base64url JSON>` of one.
//!
//! This module only writes and reads the string; opening it from a link is
//! DEV-05's deep-link handler. Both carry [`SharedRecipe`]s: the name,
//! description, prompt and kind, never IDs or dates.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde::{Deserialize, Serialize};
use url::Url;

use super::{Recipe, RecipeError, RecipeKind};

/// The `format` of an exported recipes file.
pub const RECIPE_FORMAT: &str = "deino.recipes";
/// The version of the exported file format.
pub const RECIPE_FORMAT_VERSION: u32 = 1;
/// What every share string starts with.
pub const LINK_PREFIX: &str = "deino://recipe?data=";
/// The longest share string or file read, in bytes.
const MAX_INPUT_BYTES: usize = 512 * 1024;

/// A recipe as it travels: no ID, no dates.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SharedRecipe {
    /// Its name.
    pub name: String,
    /// One line on what it does.
    #[serde(default)]
    pub description: String,
    /// What it asks the model.
    pub prompt: String,
    /// What it runs on; single when absent.
    #[serde(default)]
    pub kind: RecipeKind,
}

impl From<&Recipe> for SharedRecipe {
    fn from(recipe: &Recipe) -> Self {
        Self {
            name: recipe.name.clone(),
            description: recipe.description.clone(),
            prompt: recipe.prompt.clone(),
            kind: recipe.kind,
        }
    }
}

/// An exported recipes file.
#[derive(Serialize, Deserialize)]
struct RecipeFile {
    format: String,
    version: u32,
    recipes: Vec<SharedRecipe>,
}

/// `recipes` as a pretty-printed recipes file.
pub fn export_json(recipes: &[Recipe]) -> String {
    let file = RecipeFile {
        format: RECIPE_FORMAT.to_owned(),
        version: RECIPE_FORMAT_VERSION,
        recipes: recipes.iter().map(SharedRecipe::from).collect(),
    };
    serde_json::to_string_pretty(&file).unwrap_or_default()
}

/// `recipe`'s share string.
pub fn share_link(recipe: &Recipe) -> String {
    let json = serde_json::to_vec(&SharedRecipe::from(recipe)).unwrap_or_default();
    format!("{LINK_PREFIX}{}", URL_SAFE_NO_PAD.encode(json))
}

/// The recipe in a share string.
pub fn parse_link(link: &str) -> Result<SharedRecipe, RecipeError> {
    let url = Url::parse(link.trim()).map_err(|_| RecipeError::Unreadable("not a link"))?;
    if url.scheme() != "deino" || url.host_str() != Some("recipe") {
        return Err(RecipeError::Unreadable("not a recipe link"));
    }
    let data = url
        .query_pairs()
        .find_map(|(key, value)| (key == "data").then(|| value.into_owned()))
        .ok_or(RecipeError::Unreadable("the link carries no recipe"))?;
    let bytes = URL_SAFE_NO_PAD
        .decode(data.trim_end_matches('='))
        .map_err(|_| RecipeError::Unreadable("the link is damaged"))?;
    serde_json::from_slice(&bytes).map_err(|_| RecipeError::Unreadable("the link is damaged"))
}

/// The recipes in what the user pasted or picked: a share string, a recipes
/// file, or one recipe as JSON.
pub fn parse_shared(text: &str) -> Result<Vec<SharedRecipe>, RecipeError> {
    let text = text.trim();
    if text.len() > MAX_INPUT_BYTES {
        return Err(RecipeError::Unreadable("it is too large"));
    }
    if text.starts_with("deino:") {
        return parse_link(text).map(|recipe| vec![recipe]);
    }
    if let Ok(file) = serde_json::from_str::<RecipeFile>(text) {
        return if file.format == RECIPE_FORMAT {
            Ok(file.recipes)
        } else {
            Err(RecipeError::Unreadable("the file is not a recipes file"))
        };
    }
    serde_json::from_str::<SharedRecipe>(text)
        .map(|recipe| vec![recipe])
        .map_err(|_| RecipeError::Unreadable("expected a recipes file or a recipe link"))
}

#[cfg(test)]
mod tests {
    use chrono::DateTime;
    use deino_domain::{FakeClock, RecipeId};

    use super::*;

    fn recipe() -> Recipe {
        let clock = FakeClock::new(DateTime::UNIX_EPOCH);
        Recipe {
            id: RecipeId::generate(&clock),
            slug: "coach-me".into(),
            name: "Coach me ✨".into(),
            description: "Feedback".into(),
            prompt: "Coach me on \"pacing\" & clarity.".into(),
            kind: RecipeKind::Single,
            built_in: true,
            created_at: DateTime::UNIX_EPOCH,
            updated_at: DateTime::UNIX_EPOCH,
        }
    }

    #[test]
    fn a_share_string_round_trips() {
        let link = share_link(&recipe());

        let data = link.strip_prefix(LINK_PREFIX).unwrap();
        assert!(!data.contains('+') && !data.contains('/') && !data.contains('='));
        assert_eq!(parse_link(&link).unwrap(), SharedRecipe::from(&recipe()));
    }

    #[test]
    fn an_exported_file_imports_again() {
        let json = export_json(&[recipe(), recipe()]);

        assert!(json.contains("\"format\": \"deino.recipes\""));
        assert_eq!(parse_shared(&json).unwrap().len(), 2);
    }

    #[test]
    fn one_recipe_as_json_and_a_link_both_import() {
        let one = r#"{"name":"Mine","prompt":"Do it"}"#;

        assert_eq!(parse_shared(one).unwrap()[0].kind, RecipeKind::Single);
        assert_eq!(parse_shared(&share_link(&recipe())).unwrap().len(), 1);
    }

    #[test]
    fn other_links_and_damaged_data_are_refused() {
        for text in [
            "https://example.com/recipe?data=abc",
            "deino://meeting?data=abc",
            "deino://recipe",
            "deino://recipe?data=%%%",
            "deino://recipe?data=bm90IGpzb24",
            r#"{"format":"deino.template","version":1,"recipes":[]}"#,
            "hello",
        ] {
            assert!(parse_shared(text).is_err(), "{text}");
        }
    }
}
