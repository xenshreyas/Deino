//! The legacy summary's one adapter onto NOT-04 templates.
//!
//! `summary/service.rs` (legacy) asks for a template by the ID the UI sent;
//! this reads it from the `templates` table through `TemplateService` and
//! renders the instructions and skeleton `deino_core::summaries::SummaryRequest`
//! takes. A pre-NOT-04 ID (`daily_standup`) resolves to its successor, so an
//! old UI state or a queued job still works. The user's profile reaches the
//! prompt through the gateway's system context, not through here.

use std::sync::Arc;

use deino_core::templates::{SummaryTemplate, TemplateService, successor_of_legacy};
use deino_domain::TemplateId;
use tauri::{AppHandle, Manager, Runtime};

/// The instructions and skeleton of the template `id`, as a user-safe error
/// string for the legacy job on failure.
pub(crate) async fn summary_template<R: Runtime>(
    app: &AppHandle<R>,
    id: &str,
) -> Result<SummaryTemplate, String> {
    let templates = app
        .try_state::<Arc<TemplateService>>()
        .ok_or("Templates are not ready yet; try again in a moment")?;
    let id = TemplateId::new(successor_of_legacy(id).unwrap_or(id));
    templates
        .summary_template(&id)
        .await
        .map_err(|error| error.to_string())
}
