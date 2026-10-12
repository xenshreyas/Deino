//! Recipes (AI-04, Granola §5.3): saved prompts the user runs on a meeting,
//! by typing `/slug` in the "Ask about this meeting" bar or from the Recipes
//! page's Preview.
//!
//! Recipes are static prompts (no variables, as in Granola) and chain inside
//! a thread: a later recipe sees the earlier answers. Single-meeting only for
//! now (D13): the built-ins that need several meetings ("Weekly recap", "How
//! have I changed", "Prep my day") are listed as WIP and cannot run.
//!
//! | File | Role |
//! |---|---|
//! | `model.rs` | [`Recipe`], [`RecipeKind`], [`RecipeDraft`] |
//! | `library.rs` | [`LIBRARY`]: the built-in recipes |
//! | `slug.rs` | names to `/slugs`, unique |
//! | `share.rs` | JSON export and import, the `deino://recipe?data=` share string |
//! | `repo.rs` | [`RecipeRepo`], [`SqliteRecipeRepo`] |
//! | `service.rs` | [`RecipeService`]: list, create, edit, remix, delete, share |

mod error;
mod library;
mod model;
mod repo;
#[cfg(test)]
mod repo_tests;
mod service;
#[cfg(test)]
mod service_tests;
mod share;
mod slug;

pub use error::RecipeError;
pub use library::{BuiltinRecipe, LIBRARY};
pub use model::{
    MAX_DESCRIPTION_CHARS, MAX_NAME_CHARS, MAX_PROMPT_CHARS, Recipe, RecipeDraft, RecipeKind,
};
pub use repo::{RecipeRepo, SqliteRecipeRepo};
pub use service::{RecipeLookup, RecipeService};
pub use share::{
    LINK_PREFIX, RECIPE_FORMAT, RECIPE_FORMAT_VERSION, SharedRecipe, export_json, parse_link,
    parse_shared, share_link,
};
pub use slug::{MAX_SLUG_CHARS, is_valid_slug, slugify};
