//! Wiring for suggested follow-up emails (NOT-07): the `FollowUpService`
//! the `followups` plugin reads (SQLite, Mail through the system's URL
//! opener, the clipboard), and the generator behind the `follow_up_draft`
//! stage over the gateway's `followup.email` route, the profile's domains
//! and NOT-02's material, redacted when the setting is on (NOT-09).

use std::sync::Arc;

use async_trait::async_trait;
use deino_core::enhance::MaterialSources;
use deino_core::followups::{
    FollowUpDeps, FollowUpDrafter, FollowUpGenerator, FollowUpModel, FollowUpPorts,
    FollowUpService, GatewayFollowUpModel, SqliteFollowUpRepo,
};
use deino_core::redaction::{LocalModelCheck, Redactor, SettingRedactor, TextRedactor};
use deino_db::Database;
use deino_llm::followup::{FollowUpDecision, FollowUpDraftAnswer};
use deino_llm::{Gateway, LlmError, Prompt, ProviderId};
use tauri::{AppHandle, Manager};

use super::action_items::ActionItemWiring;
use crate::features::export::{SystemClipboard, SystemOpener};

/// Manages `Arc<FollowUpService>` and returns the stage's drafter.
pub(super) fn attach_followups(
    app: &AppHandle,
    database: &Database,
    wiring: &ActionItemWiring,
) -> Arc<dyn FollowUpDrafter> {
    let store = Arc::new(SqliteFollowUpRepo::new(database.clone()));
    app.manage(Arc::new(FollowUpService::new(FollowUpPorts {
        store: store.clone(),
        opener: Arc::new(SystemOpener),
        clipboard: Arc::new(SystemClipboard),
        clock: Arc::clone(&wiring.clock),
    })));
    let mut sources = MaterialSources::sqlite(database);
    let model: Arc<dyn FollowUpModel> = match &wiring.gateway {
        Some(gateway) => {
            sources = sources.with_redactor(redactor(wiring, gateway));
            Arc::new(GatewayFollowUpModel::new(Arc::clone(gateway)))
        }
        None => Arc::new(Unwired),
    };
    Arc::new(FollowUpGenerator::new(FollowUpDeps {
        store: store.clone(),
        participants: store,
        sources,
        domains: wiring.settings.clone(),
        model,
        clock: Arc::clone(&wiring.clock),
    }))
}

/// Hides secrets when "Hide sensitive information" is on, as for notes.
fn redactor(wiring: &ActionItemWiring, gateway: &Arc<Gateway>) -> Arc<dyn TextRedactor> {
    let check = Arc::new(LocalModelCheck::new(Arc::clone(gateway)));
    Arc::new(SettingRedactor::new(
        wiring.settings.clone(),
        Arc::new(Redactor::with_check(check)),
    ))
}

/// No model could be wired at startup: drafting fails as unavailable, and
/// the stage waits for the user.
struct Unwired;

impl Unwired {
    fn unavailable() -> LlmError {
        LlmError::Unavailable {
            provider: ProviderId::BuiltIn,
            reason: "no model is set up for follow-up emails".to_owned(),
        }
    }
}

#[async_trait]
impl FollowUpModel for Unwired {
    async fn decide(&self, _prompt: Prompt) -> Result<FollowUpDecision, LlmError> {
        Err(Self::unavailable())
    }

    async fn draft(&self, _prompt: Prompt) -> Result<FollowUpDraftAnswer, LlmError> {
        Err(Self::unavailable())
    }
}
