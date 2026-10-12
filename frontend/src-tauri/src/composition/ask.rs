//! Wiring for "Ask about this meeting" (AI-02-lite, D13) and recipes
//! (AI-04): the `RecipeService` the `recipes` plugin calls, and the
//! `AskService` the `ask` plugin calls, over SQLite and the gateway's `chat`
//! route, reading the meeting through `MaterialSources` with redaction when
//! the setting is on (NOT-09). Without a gateway every question fails as
//! unavailable; recipes still work.

use std::sync::Arc;

use deino_core::ask::{AskDeps, AskModel, AskService, GatewayAskModel, NoAskModel, SqliteAskRepo};
use deino_core::enhance::MaterialSources;
use deino_core::recipes::{RecipeService, SqliteRecipeRepo};
use deino_core::redaction::{LocalModelCheck, Redactor, SettingRedactor, TextRedactor};
use deino_db::Database;
use deino_llm::Gateway;
use tauri::{AppHandle, Manager};

use super::action_items::ActionItemWiring;
use super::attach::Phase;

super::attach::attach!(
    "ask",
    20,
    Phase::ReadModel(|cx, ai| attach_ask(cx.app, cx.database, ai))
);

/// Manages `Arc<RecipeService>` and `Arc<AskService>`.
fn attach_ask(app: &AppHandle, database: &Database, wiring: &ActionItemWiring) {
    let recipes = Arc::new(RecipeService::new(
        Arc::new(SqliteRecipeRepo::new(database.clone())),
        Arc::clone(&wiring.clock),
    ));
    app.manage(Arc::clone(&recipes));
    let sources = MaterialSources::sqlite(database);
    let (sources, model): (MaterialSources, Arc<dyn AskModel>) = match &wiring.gateway {
        Some(gateway) => (
            sources.with_redactor(redactor(wiring, gateway)),
            Arc::new(GatewayAskModel::new(Arc::clone(gateway))),
        ),
        None => (sources, Arc::new(NoAskModel)),
    };
    app.manage(Arc::new(AskService::new(AskDeps {
        sources,
        store: Arc::new(SqliteAskRepo::new(database.clone())),
        recipes,
        model,
        clock: Arc::clone(&wiring.clock),
    })));
}

/// Hides secrets when "Hide sensitive information" is on, as for notes.
fn redactor(wiring: &ActionItemWiring, gateway: &Arc<Gateway>) -> Arc<dyn TextRedactor> {
    let check = Arc::new(LocalModelCheck::new(Arc::clone(gateway)));
    Arc::new(SettingRedactor::new(
        wiring.settings.clone(),
        Arc::new(Redactor::with_check(check)),
    ))
}
