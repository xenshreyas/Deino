//! [`RecipeRepo`]: recipes in the `recipes` table.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_db::{Database, DbError};
use deino_domain::RecipeId;

use super::{Recipe, RecipeKind};

/// Reads and writes recipes.
#[async_trait]
pub trait RecipeRepo: Send + Sync {
    /// Every recipe: the built-ins in library order, then the user's, oldest first.
    async fn list(&self) -> Result<Vec<Recipe>, DbError>;

    /// One recipe.
    async fn find(&self, id: &RecipeId) -> Result<Option<Recipe>, DbError>;

    /// Every slug in use.
    async fn slugs(&self) -> Result<Vec<String>, DbError>;

    /// Stores a new recipe.
    async fn insert(&self, recipe: &Recipe) -> Result<(), DbError>;

    /// Rewrites one of the user's recipes; built-ins are left alone.
    async fn update(&self, recipe: &Recipe) -> Result<(), DbError>;

    /// Deletes one of the user's recipes; false when there was none.
    async fn delete(&self, id: &RecipeId) -> Result<bool, DbError>;

    /// Writes the library: each built-in inserted or brought up to date by
    /// slug (a user's recipe holding the slug wins), and built-ins the
    /// library no longer has removed.
    async fn sync_built_ins(&self, library: &[Recipe]) -> Result<(), DbError>;
}

/// [`RecipeRepo`] over Deino's SQLite database.
#[derive(Clone, Debug)]
pub struct SqliteRecipeRepo {
    database: Database,
}

impl SqliteRecipeRepo {
    /// A repository over an open, migrated database.
    pub fn new(database: Database) -> Self {
        Self { database }
    }
}

/// The columns, typed.
macro_rules! select_recipes {
    ($tail:literal $(, $arg:expr)*) => {
        sqlx::query_as!(
            Recipe,
            r#"SELECT id AS "id!: RecipeId", slug AS "slug!", name AS "name!",
                description AS "description!", prompt AS "prompt!",
                kind AS "kind!: RecipeKind", built_in AS "built_in!: bool",
                created_at AS "created_at!: DateTime<Utc>",
                updated_at AS "updated_at!: DateTime<Utc>"
            FROM recipes "# + $tail
            $(, $arg)*
        )
    };
}

#[async_trait]
impl RecipeRepo for SqliteRecipeRepo {
    async fn list(&self) -> Result<Vec<Recipe>, DbError> {
        let recipes = select_recipes!("ORDER BY built_in DESC, created_at, name")
            .fetch_all(self.database.reader())
            .await?;
        Ok(recipes)
    }

    async fn find(&self, id: &RecipeId) -> Result<Option<Recipe>, DbError> {
        let recipe = select_recipes!("WHERE id = ?1", id)
            .fetch_optional(self.database.reader())
            .await?;
        Ok(recipe)
    }

    async fn slugs(&self) -> Result<Vec<String>, DbError> {
        let slugs = sqlx::query_scalar!(r#"SELECT slug AS "slug!" FROM recipes"#)
            .fetch_all(self.database.reader())
            .await?;
        Ok(slugs)
    }

    async fn insert(&self, recipe: &Recipe) -> Result<(), DbError> {
        sqlx::query!(
            "INSERT INTO recipes
                (id, slug, name, description, prompt, kind, built_in, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            recipe.id,
            recipe.slug,
            recipe.name,
            recipe.description,
            recipe.prompt,
            recipe.kind,
            recipe.built_in,
            recipe.created_at,
            recipe.updated_at,
        )
        .execute(self.database.writer())
        .await?;
        Ok(())
    }

    async fn update(&self, recipe: &Recipe) -> Result<(), DbError> {
        sqlx::query!(
            "UPDATE recipes SET slug = ?2, name = ?3, description = ?4, prompt = ?5,
                 updated_at = ?6
             WHERE id = ?1 AND built_in = 0",
            recipe.id,
            recipe.slug,
            recipe.name,
            recipe.description,
            recipe.prompt,
            recipe.updated_at,
        )
        .execute(self.database.writer())
        .await?;
        Ok(())
    }

    async fn delete(&self, id: &RecipeId) -> Result<bool, DbError> {
        let done = sqlx::query!("DELETE FROM recipes WHERE id = ?1 AND built_in = 0", id)
            .execute(self.database.writer())
            .await?;
        Ok(done.rows_affected() > 0)
    }

    async fn sync_built_ins(&self, library: &[Recipe]) -> Result<(), DbError> {
        let mut tx = self.database.writer().begin().await?;
        for recipe in library {
            sqlx::query!(
                "INSERT INTO recipes
                    (id, slug, name, description, prompt, kind, built_in, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, ?7, ?7)
                 ON CONFLICT (slug) DO UPDATE SET
                     name = excluded.name, description = excluded.description,
                     prompt = excluded.prompt, kind = excluded.kind,
                     updated_at = excluded.updated_at
                 WHERE recipes.built_in = 1 AND (recipes.name, recipes.description,
                     recipes.prompt, recipes.kind) IS NOT (excluded.name,
                     excluded.description, excluded.prompt, excluded.kind)",
                recipe.id,
                recipe.slug,
                recipe.name,
                recipe.description,
                recipe.prompt,
                recipe.kind,
                recipe.created_at,
            )
            .execute(&mut *tx)
            .await?;
        }
        let keep = serde_json::to_string(&library.iter().map(|r| &r.slug).collect::<Vec<_>>())
            .unwrap_or_default();
        sqlx::query!(
            "DELETE FROM recipes WHERE built_in = 1
             AND slug NOT IN (SELECT value FROM json_each(?1))",
            keep,
        )
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }
}
