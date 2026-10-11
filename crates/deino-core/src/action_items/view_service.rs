//! [`SavedViewService`]: saving, renaming and deleting the Action Items
//! page's views, with the name rules.

use std::sync::Arc;

use deino_domain::{ActionItemViewId, Clock};

use super::{ActionItemError, SavedView, SavedViewInput, SavedViewRepo};

/// The longest view name, in characters: a tab label.
pub const MAX_VIEW_NAME_CHARS: usize = 40;

/// Saves and lists the page's views.
pub struct SavedViewService {
    repo: Arc<dyn SavedViewRepo>,
    clock: Arc<dyn Clock>,
}

impl SavedViewService {
    /// The service over its store.
    pub fn new(repo: Arc<dyn SavedViewRepo>, clock: Arc<dyn Clock>) -> Self {
        Self { repo, clock }
    }

    /// Every view, in tab order.
    pub async fn list(&self) -> Result<Vec<SavedView>, ActionItemError> {
        Ok(self.repo.list_views().await?)
    }

    /// Saves a new view after the last tab.
    pub async fn create(&self, input: SavedViewInput) -> Result<SavedView, ActionItemError> {
        let name = self.free_name(&input.name, None).await?;
        let now = self.clock.now();
        let view = SavedView {
            id: ActionItemViewId::generate(self.clock.as_ref()),
            name,
            filter: input.filter,
            position: 0,
            created_at: now,
            updated_at: now,
        };
        self.repo.create_view(&view).await?;
        self.find(view.id).await
    }

    /// Renames a view or replaces its filter.
    pub async fn update(
        &self,
        id: ActionItemViewId,
        input: SavedViewInput,
    ) -> Result<SavedView, ActionItemError> {
        let mut view = self.find(id).await?;
        view.name = self.free_name(&input.name, Some(id)).await?;
        view.filter = input.filter;
        view.updated_at = self.clock.now();
        self.repo.update_view(&view).await?;
        Ok(view)
    }

    /// Deletes a view.
    pub async fn delete(&self, id: ActionItemViewId) -> Result<(), ActionItemError> {
        if self.repo.delete_view(&id).await? {
            Ok(())
        } else {
            Err(ActionItemError::ViewNotFound(id))
        }
    }

    async fn find(&self, id: ActionItemViewId) -> Result<SavedView, ActionItemError> {
        self.list()
            .await?
            .into_iter()
            .find(|view| view.id == id)
            .ok_or(ActionItemError::ViewNotFound(id))
    }

    /// `name` trimmed, if it is a valid label no other view has.
    async fn free_name(
        &self,
        name: &str,
        renaming: Option<ActionItemViewId>,
    ) -> Result<String, ActionItemError> {
        let name = name.trim();
        if name.is_empty() {
            return Err(ActionItemError::invalid("name", "cannot be empty"));
        }
        if name.chars().count() > MAX_VIEW_NAME_CHARS {
            return Err(ActionItemError::invalid("name", "is too long"));
        }
        let taken = self.list().await?.into_iter().any(|view| {
            Some(view.id) != renaming && view.name.to_lowercase() == name.to_lowercase()
        });
        if taken {
            return Err(ActionItemError::DuplicateViewName);
        }
        Ok(name.to_owned())
    }
}
