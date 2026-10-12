//! [`RecipeService`]: the rules over [`RecipeRepo`]: the library is written
//! on first use, built-ins are read-only (remix one to change it), every
//! slug is unique, and only single-meeting recipes are created or imported.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::Duration;
use deino_domain::{Clock, RecipeId};
use tokio::sync::OnceCell;

use super::share::{SharedRecipe, export_json, parse_shared, share_link};
use super::slug::{slugify, unique_slug};
use super::{LIBRARY, Recipe, RecipeDraft, RecipeError, RecipeKind, RecipeRepo};

/// A remix's name: the original's with this after it.
const REMIX_SUFFIX: &str = " (remix)";

/// Finds the recipe a question runs (the ask service's port).
#[async_trait]
pub trait RecipeLookup: Send + Sync {
    /// The recipe, if it can run on one meeting.
    async fn runnable(&self, id: &RecipeId) -> Result<Recipe, RecipeError>;
}

/// Recipes: the library and the user's own.
pub struct RecipeService {
    repo: Arc<dyn RecipeRepo>,
    clock: Arc<dyn Clock>,
    synced: OnceCell<()>,
}

impl RecipeService {
    /// A service over `repo`.
    pub fn new(repo: Arc<dyn RecipeRepo>, clock: Arc<dyn Clock>) -> Self {
        Self {
            repo,
            clock,
            synced: OnceCell::new(),
        }
    }

    /// Every recipe: the built-ins in library order, then the user's.
    pub async fn list(&self) -> Result<Vec<Recipe>, RecipeError> {
        self.ensure_library().await?;
        Ok(self.repo.list().await?)
    }

    /// One recipe.
    pub async fn get(&self, id: &RecipeId) -> Result<Recipe, RecipeError> {
        self.ensure_library().await?;
        self.repo.find(id).await?.ok_or(RecipeError::NotFound(*id))
    }

    /// Saves a new recipe of the user's.
    pub async fn create(&self, draft: RecipeDraft) -> Result<Recipe, RecipeError> {
        let draft = draft.validated()?;
        self.ensure_library().await?;
        let slug = unique_slug(&slugify(&draft.name), &self.repo.slugs().await?);
        let now = self.clock.now();
        let recipe = Recipe {
            id: RecipeId::generate(self.clock.as_ref()),
            slug,
            name: draft.name,
            description: draft.description,
            prompt: draft.prompt,
            kind: RecipeKind::Single,
            built_in: false,
            created_at: now,
            updated_at: now,
        };
        self.repo.insert(&recipe).await?;
        Ok(recipe)
    }

    /// Rewrites one of the user's recipes; its slug follows its name.
    pub async fn update(&self, id: &RecipeId, draft: RecipeDraft) -> Result<Recipe, RecipeError> {
        let current = self.get(id).await?;
        if current.built_in {
            return Err(RecipeError::BuiltIn);
        }
        let draft = draft.validated()?;
        let mut taken = self.repo.slugs().await?;
        taken.retain(|slug| *slug != current.slug);
        let recipe = Recipe {
            slug: unique_slug(&slugify(&draft.name), &taken),
            name: draft.name,
            description: draft.description,
            prompt: draft.prompt,
            updated_at: self.clock.now(),
            ..current
        };
        self.repo.update(&recipe).await?;
        Ok(recipe)
    }

    /// Deletes one of the user's recipes.
    pub async fn delete(&self, id: &RecipeId) -> Result<(), RecipeError> {
        if self.get(id).await?.built_in {
            return Err(RecipeError::BuiltIn);
        }
        self.repo.delete(id).await?;
        Ok(())
    }

    /// An editable copy of any single-meeting recipe (Granola's Remix).
    pub async fn remix(&self, id: &RecipeId) -> Result<Recipe, RecipeError> {
        let original = self.get(id).await?;
        if !original.is_runnable() {
            return Err(RecipeError::MultiMeeting);
        }
        let name: String = original
            .name
            .chars()
            .take(super::MAX_NAME_CHARS - 8)
            .collect();
        self.create(RecipeDraft {
            name: format!("{name}{REMIX_SUFFIX}"),
            description: original.description,
            prompt: original.prompt,
        })
        .await
    }

    /// The recipes `ids` names (every one of the user's when empty) as a
    /// recipes file.
    pub async fn export(&self, ids: &[RecipeId]) -> Result<String, RecipeError> {
        let all = self.list().await?;
        let chosen: Vec<Recipe> = if ids.is_empty() {
            all.into_iter().filter(|recipe| !recipe.built_in).collect()
        } else {
            all.into_iter()
                .filter(|recipe| ids.contains(&recipe.id))
                .collect()
        };
        Ok(export_json(&chosen))
    }

    /// `id`'s share string, `deino://recipe?data=…`.
    pub async fn share_link(&self, id: &RecipeId) -> Result<String, RecipeError> {
        Ok(share_link(&self.get(id).await?))
    }

    /// Saves the recipes in a pasted file or share string as the user's own.
    pub async fn import(&self, text: &str) -> Result<Vec<Recipe>, RecipeError> {
        let shared = parse_shared(text)?;
        if shared.is_empty() {
            return Err(RecipeError::Unreadable("it holds no recipes"));
        }
        let drafts = shared
            .into_iter()
            .map(to_draft)
            .collect::<Result<Vec<_>, _>>()?;
        let mut saved = Vec::with_capacity(drafts.len());
        for draft in drafts {
            saved.push(self.create(draft).await?);
        }
        Ok(saved)
    }

    /// Writes the library once per run.
    async fn ensure_library(&self) -> Result<(), RecipeError> {
        self.synced
            .get_or_try_init(|| async {
                let library = self.library();
                self.repo.sync_built_ins(&library).await
            })
            .await?;
        Ok(())
    }

    /// The library as rows, each a millisecond apart so they list in order.
    fn library(&self) -> Vec<Recipe> {
        let start = self.clock.now();
        (0_i64..)
            .zip(LIBRARY)
            .map(|(index, builtin)| Recipe {
                id: RecipeId::generate(self.clock.as_ref()),
                slug: builtin.slug.to_owned(),
                name: builtin.name.to_owned(),
                description: builtin.description.to_owned(),
                prompt: builtin.prompt.to_owned(),
                kind: builtin.kind,
                built_in: true,
                created_at: start + Duration::milliseconds(index),
                updated_at: start,
            })
            .collect()
    }
}

/// A shared recipe as a draft, refused when it needs several meetings.
fn to_draft(shared: SharedRecipe) -> Result<RecipeDraft, RecipeError> {
    if shared.kind == RecipeKind::Multi {
        return Err(RecipeError::MultiMeeting);
    }
    RecipeDraft {
        name: shared.name,
        description: shared.description,
        prompt: shared.prompt,
    }
    .validated()
}

#[async_trait]
impl RecipeLookup for RecipeService {
    async fn runnable(&self, id: &RecipeId) -> Result<Recipe, RecipeError> {
        let recipe = self.get(id).await?;
        if recipe.is_runnable() {
            Ok(recipe)
        } else {
            Err(RecipeError::MultiMeeting)
        }
    }
}
