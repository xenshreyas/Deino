//! Commands of the `recipes` plugin: parse input, call one service method,
//! map the error. No logic lives here.

#![allow(
    clippy::let_underscore_must_use,
    reason = "#[tauri::command] on an async fn with a State<'_> argument expands to a `let _` type check of its Result"
)]

use std::sync::Arc;

use deino_core::recipes::{Recipe, RecipeDraft, RecipeService};
use deino_domain::{AppError, RecipeId};
use tauri::{Runtime, State};

use crate::error::IntoIpc;

/// The recipe service, as Tauri state.
type Recipes<'a> = State<'a, Arc<RecipeService>>;

/// The plugin's commands, collected here so they can stay private.
pub(super) fn collect<R: Runtime>() -> tauri_specta::Commands<R> {
    tauri_specta::collect_commands![
        list,
        create,
        update,
        remove,
        remix,
        export_json,
        import_shared,
        share_link
    ]
}

/// Every recipe: the built-ins in library order, then the user's.
#[tauri::command]
#[specta::specta]
async fn list(recipes: Recipes<'_>) -> Result<Vec<Recipe>, AppError> {
    recipes.list().await.into_ipc()
}

/// Saves a new recipe; its `/slug` comes from its name.
#[tauri::command]
#[specta::specta]
async fn create(recipes: Recipes<'_>, draft: RecipeDraft) -> Result<Recipe, AppError> {
    recipes.create(draft).await.into_ipc()
}

/// Rewrites one of the user's recipes.
#[tauri::command]
#[specta::specta]
async fn update(
    recipes: Recipes<'_>,
    id: RecipeId,
    draft: RecipeDraft,
) -> Result<Recipe, AppError> {
    recipes.update(&id, draft).await.into_ipc()
}

/// Deletes one of the user's recipes.
#[tauri::command]
#[specta::specta]
async fn remove(recipes: Recipes<'_>, id: RecipeId) -> Result<(), AppError> {
    recipes.delete(&id).await.into_ipc()
}

/// An editable copy of a recipe.
#[tauri::command]
#[specta::specta]
async fn remix(recipes: Recipes<'_>, id: RecipeId) -> Result<Recipe, AppError> {
    recipes.remix(&id).await.into_ipc()
}

/// The recipes `ids` names (the user's own when empty) as a recipes file.
#[tauri::command]
#[specta::specta]
async fn export_json(recipes: Recipes<'_>, ids: Vec<RecipeId>) -> Result<String, AppError> {
    recipes.export(&ids).await.into_ipc()
}

/// Saves the recipes in a pasted file or `deino://recipe` share string.
#[tauri::command]
#[specta::specta]
async fn import_shared(recipes: Recipes<'_>, text: String) -> Result<Vec<Recipe>, AppError> {
    recipes.import(&text).await.into_ipc()
}

/// The recipe's share string, `deino://recipe?data=…`.
#[tauri::command]
#[specta::specta]
async fn share_link(recipes: Recipes<'_>, id: RecipeId) -> Result<String, AppError> {
    recipes.share_link(&id).await.into_ipc()
}
