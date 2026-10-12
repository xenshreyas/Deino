//! [`MeetingViewService`]: saving, renaming and deleting the Meetings
//! page's views, with the name rules.

use std::sync::Arc;

use deino_domain::{Clock, MeetingViewId};

use super::{MeetingError, MeetingView, MeetingViewInput, MeetingViewRepo};

/// The longest view name, in characters: a tab label.
pub const MAX_VIEW_NAME_CHARS: usize = 40;

/// Saves and lists the page's views.
pub struct MeetingViewService {
    repo: Arc<dyn MeetingViewRepo>,
    clock: Arc<dyn Clock>,
}

impl MeetingViewService {
    /// The service over its store.
    pub fn new(repo: Arc<dyn MeetingViewRepo>, clock: Arc<dyn Clock>) -> Self {
        Self { repo, clock }
    }

    /// Every view, in tab order.
    pub async fn list(&self) -> Result<Vec<MeetingView>, MeetingError> {
        Ok(self.repo.list_views().await?)
    }

    /// Saves a new view after the last tab.
    pub async fn create(&self, input: MeetingViewInput) -> Result<MeetingView, MeetingError> {
        let name = self.free_name(&input.name, None).await?;
        let now = self.clock.now();
        let view = MeetingView {
            id: MeetingViewId::generate(self.clock.as_ref()),
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
        id: MeetingViewId,
        input: MeetingViewInput,
    ) -> Result<MeetingView, MeetingError> {
        let mut view = self.find(id).await?;
        view.name = self.free_name(&input.name, Some(id)).await?;
        view.filter = input.filter;
        view.updated_at = self.clock.now();
        self.repo.update_view(&view).await?;
        Ok(view)
    }

    /// Deletes a view.
    pub async fn delete(&self, id: MeetingViewId) -> Result<(), MeetingError> {
        if self.repo.delete_view(&id).await? {
            Ok(())
        } else {
            Err(MeetingError::ViewNotFound(id))
        }
    }

    async fn find(&self, id: MeetingViewId) -> Result<MeetingView, MeetingError> {
        self.list()
            .await?
            .into_iter()
            .find(|view| view.id == id)
            .ok_or(MeetingError::ViewNotFound(id))
    }

    /// `name` trimmed, if it is a valid label no other view has.
    async fn free_name(
        &self,
        name: &str,
        renaming: Option<MeetingViewId>,
    ) -> Result<String, MeetingError> {
        let name = name.trim();
        if name.is_empty() {
            return Err(MeetingError::InvalidViewName("cannot be empty"));
        }
        if name.chars().count() > MAX_VIEW_NAME_CHARS {
            return Err(MeetingError::InvalidViewName("is too long"));
        }
        let taken = self.list().await?.into_iter().any(|view| {
            Some(view.id) != renaming && view.name.to_lowercase() == name.to_lowercase()
        });
        if taken {
            return Err(MeetingError::DuplicateViewName);
        }
        Ok(name.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use deino_db::test_db;
    use deino_domain::FakeClock;
    use deino_testkit::at;

    use super::*;
    use crate::meetings::{MeetingFilter, SqliteMeetingRepo};

    async fn service() -> MeetingViewService {
        let repo = Arc::new(SqliteMeetingRepo::new(test_db().await));
        let clock = Arc::new(FakeClock::new(at("2026-10-12T09:00:00Z")));
        MeetingViewService::new(repo, clock)
    }

    fn input(name: &str) -> MeetingViewInput {
        let filter = MeetingFilter {
            has_action_items: true,
            ..MeetingFilter::default()
        };
        MeetingViewInput {
            name: name.into(),
            filter,
        }
    }

    #[tokio::test]
    async fn views_are_appended_as_tabs_and_keep_their_typed_filter() {
        let views = service().await;

        views.create(input(" Customers ")).await.unwrap();
        let second = views.create(input("Hiring")).await.unwrap();

        let listed = views.list().await.unwrap();
        let names: Vec<&str> = listed.iter().map(|view| view.name.as_str()).collect();
        assert_eq!(names, ["Customers", "Hiring"]);
        assert_eq!(second.position, 1);
        assert!(second.filter.has_action_items);
    }

    #[tokio::test]
    async fn names_are_unique_ignoring_case_and_must_fit_a_tab() {
        let views = service().await;
        let first = views.create(input("Customers")).await.unwrap();

        let duplicate = views.create(input("customers")).await.unwrap_err();
        let blank = views.create(input("  ")).await.unwrap_err();
        let long = views
            .create(input(&"x".repeat(MAX_VIEW_NAME_CHARS + 1)))
            .await
            .unwrap_err();
        let renamed = views.update(first.id, input("CUSTOMERS")).await.unwrap();

        assert!(matches!(duplicate, MeetingError::DuplicateViewName));
        assert!(matches!(blank, MeetingError::InvalidViewName(_)));
        assert!(matches!(long, MeetingError::InvalidViewName(_)));
        assert_eq!(renamed.name, "CUSTOMERS", "a view may change its own case");
    }

    #[tokio::test]
    async fn deleting_a_missing_view_is_not_found() {
        let views = service().await;
        let view = views.create(input("Customers")).await.unwrap();

        views.delete(view.id).await.unwrap();

        assert!(matches!(
            views.delete(view.id).await,
            Err(MeetingError::ViewNotFound(_))
        ));
        assert!(views.list().await.unwrap().is_empty());
    }
}
