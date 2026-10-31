//! Route resolution: which provider and model serve a [`Route`].
//!
//! The user may pin any route to a provider and model in Settings → AI
//! ([`RouteOverrides`], persisted through the [`RouteStore`] port). Every
//! other route takes the default of PARITY_PLAN §4.3 / §8 D3:
//!
//! - with an Anthropic key: `claude-sonnet-5-5` for smart routes and
//!   `claude-haiku-5-5` for fast ones;
//! - without one: the built-in local model;
//! - `embed` is always the local embedding model.
//!
//! OpenAI and the other providers are opt-in per route, from the provider's
//! live model list.

use std::collections::BTreeMap;
use std::error::Error;
use std::sync::{Arc, Mutex, PoisonError};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::{CredentialSource, LlmError, LocalModels, ProviderId, Route, RouteTier, Target};

/// Claude's default model for smart routes (§4.3).
pub const CLAUDE_SMART_MODEL: &str = "claude-sonnet-5-5";
/// Claude's default model for fast routes (§4.3).
pub const CLAUDE_FAST_MODEL: &str = "claude-haiku-5-5";
/// The local embedding model for `embed` (§4.4), served by llama-helper.
pub const LOCAL_EMBEDDING_MODEL: &str = "nomic-embed-text-v1.5";

/// The routes the user pinned, by route.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RouteOverrides(BTreeMap<Route, Target>);

impl From<BTreeMap<Route, Target>> for RouteOverrides {
    fn from(map: BTreeMap<Route, Target>) -> Self {
        Self(map)
    }
}

impl From<RouteOverrides> for BTreeMap<Route, Target> {
    fn from(overrides: RouteOverrides) -> Self {
        overrides.0
    }
}

impl RouteOverrides {
    /// The pinned target of `route`, if any.
    pub fn get(&self, route: Route) -> Option<&Target> {
        self.0.get(&route)
    }

    /// Pins `route` to `target`, or resets it to the default with `None`.
    pub fn set(&mut self, route: Route, target: Option<Target>) {
        match target {
            Some(target) => self.0.insert(route, target),
            None => self.0.remove(&route),
        };
    }
}

/// Whether a route's target is the user's choice or the default.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum RouteSource {
    /// The §4.3 default for the current keys.
    Default,
    /// The user pinned it in Settings → AI.
    Custom,
}

/// A route and what serves it, as the Settings → AI table shows it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedRoute {
    /// The route.
    pub route: Route,
    /// Its tier, which picks its default.
    pub tier: RouteTier,
    /// The provider and model that serve it.
    pub target: Target,
    /// Whether the target is the user's choice.
    pub source: RouteSource,
}

/// The route settings store failed.
#[derive(Debug, thiserror::Error)]
#[error("the route settings store failed")]
pub struct RouteStoreError(#[source] pub Box<dyn Error + Send + Sync>);

/// Where the user's route choices are kept.
#[async_trait]
pub trait RouteStore: Send + Sync {
    /// The saved choices; empty when nothing was ever saved.
    async fn load(&self) -> Result<RouteOverrides, RouteStoreError>;
    /// Replaces the saved choices.
    async fn save(&self, overrides: &RouteOverrides) -> Result<(), RouteStoreError>;
}

/// Resolves routes against the user's choices and the available keys.
pub struct Router {
    store: Arc<dyn RouteStore>,
    credentials: Arc<dyn CredentialSource>,
    local: Arc<dyn LocalModels>,
}

/// What the defaults depend on, read once per resolution.
struct Defaults {
    has_claude: bool,
    local_model: String,
}

impl Router {
    /// A router over `store`. Without an Anthropic key, text routes default
    /// to the first downloaded built-in model in `local` (or its first model).
    pub fn new(
        store: Arc<dyn RouteStore>,
        credentials: Arc<dyn CredentialSource>,
        local: Arc<dyn LocalModels>,
    ) -> Self {
        Self {
            store,
            credentials,
            local,
        }
    }

    /// What serves `route` right now.
    pub async fn resolve(&self, route: Route) -> Result<ResolvedRoute, LlmError> {
        let overrides = self.store.load().await?;
        let defaults = self.defaults().await?;
        Ok(resolve_with(route, &overrides, &defaults))
    }

    /// Every route and what serves it, in [`Route::ALL`] order.
    pub async fn table(&self) -> Result<Vec<ResolvedRoute>, LlmError> {
        let overrides = self.store.load().await?;
        let defaults = self.defaults().await?;
        Ok(Route::ALL
            .into_iter()
            .map(|route| resolve_with(route, &overrides, &defaults))
            .collect())
    }

    /// Pins `route` to `target`, or resets it with `None`, and returns what
    /// serves it now. `embed` cannot be pinned: it is always local.
    pub async fn set(
        &self,
        route: Route,
        target: Option<Target>,
    ) -> Result<ResolvedRoute, LlmError> {
        if route == Route::Embed && target.is_some() {
            return Err(LlmError::Unsupported {
                provider: ProviderId::BuiltIn,
                feature: "choosing the embedding model",
            });
        }
        let mut overrides = self.store.load().await?;
        overrides.set(route, target);
        self.store.save(&overrides).await?;
        self.resolve(route).await
    }

    async fn defaults(&self) -> Result<Defaults, LlmError> {
        let key = self
            .credentials
            .api_key(ProviderId::Anthropic)
            .await
            .map_err(|source| LlmError::Credentials {
                provider: ProviderId::Anthropic,
                source,
            })?;
        let models = self.local.models().await;
        let local_model = models
            .iter()
            .find(|model| model.ready)
            .or(models.first())
            .map(|model| model.id.clone())
            .unwrap_or_default();
        Ok(Defaults {
            has_claude: key.is_some(),
            local_model,
        })
    }
}

fn resolve_with(route: Route, overrides: &RouteOverrides, defaults: &Defaults) -> ResolvedRoute {
    let (target, source) = match overrides.get(route) {
        Some(target) if route != Route::Embed => (target.clone(), RouteSource::Custom),
        _ => (default_target(route.tier(), defaults), RouteSource::Default),
    };
    ResolvedRoute {
        route,
        tier: route.tier(),
        target,
        source,
    }
}

fn default_target(tier: RouteTier, defaults: &Defaults) -> Target {
    let (provider, model) = match (tier, defaults.has_claude) {
        (RouteTier::Embedding, _) => (ProviderId::BuiltIn, LOCAL_EMBEDDING_MODEL),
        (RouteTier::Smart, true) => (ProviderId::Anthropic, CLAUDE_SMART_MODEL),
        (RouteTier::Fast, true) => (ProviderId::Anthropic, CLAUDE_FAST_MODEL),
        (RouteTier::Smart | RouteTier::Fast, false) => {
            (ProviderId::BuiltIn, defaults.local_model.as_str())
        }
    };
    Target {
        provider,
        model: model.to_owned(),
        effort: None,
    }
}
/// Route choices kept in memory, for tests and the eval harness.
#[derive(Debug, Default)]
pub struct MemoryRouteStore(Mutex<RouteOverrides>);

#[async_trait]
impl RouteStore for MemoryRouteStore {
    async fn load(&self) -> Result<RouteOverrides, RouteStoreError> {
        Ok(self
            .0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone())
    }

    async fn save(&self, overrides: &RouteOverrides) -> Result<(), RouteStoreError> {
        overrides.clone_into(&mut self.0.lock().unwrap_or_else(PoisonError::into_inner));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FakeLocalModels, StaticCredentials};

    const LOCAL: &str = "qwen3.5:2b";

    fn router(credentials: StaticCredentials) -> Router {
        Router::new(
            Arc::new(MemoryRouteStore::default()),
            Arc::new(credentials),
            Arc::new(FakeLocalModels::new().with_models(&[LOCAL, "gemma3:1b"])),
        )
    }

    fn target(provider: ProviderId, model: &str) -> Target {
        Target {
            provider,
            model: model.to_owned(),
            effort: None,
        }
    }

    #[tokio::test]
    async fn with_a_claude_key_smart_routes_use_sonnet_and_fast_routes_haiku() {
        let router = router(StaticCredentials::new().key(ProviderId::Anthropic, "sk-ant-1"));

        let notes = router.resolve(Route::NotesEnhance).await.unwrap();
        let title = router.resolve(Route::Title).await.unwrap();

        assert_eq!(
            notes.target,
            target(ProviderId::Anthropic, CLAUDE_SMART_MODEL)
        );
        assert_eq!(
            title.target,
            target(ProviderId::Anthropic, CLAUDE_FAST_MODEL)
        );
        assert_eq!(notes.source, RouteSource::Default);
    }

    #[tokio::test]
    async fn without_a_claude_key_every_text_route_falls_back_to_the_built_in_model() {
        let router = router(StaticCredentials::new().key(ProviderId::OpenAi, "sk-1"));

        let table = router.table().await.unwrap();

        assert!(
            table
                .iter()
                .filter(|row| row.route != Route::Embed)
                .all(|row| row.target == target(ProviderId::BuiltIn, LOCAL))
        );
    }

    #[tokio::test]
    async fn embed_is_always_local_and_cannot_be_pinned() {
        let router = router(StaticCredentials::new().key(ProviderId::Anthropic, "sk-ant-1"));

        let embed = router.resolve(Route::Embed).await.unwrap();
        let pin = router
            .set(
                Route::Embed,
                Some(target(ProviderId::OpenAi, "text-embedding-3-small")),
            )
            .await;

        assert_eq!(
            embed.target,
            target(ProviderId::BuiltIn, LOCAL_EMBEDDING_MODEL)
        );
        assert!(matches!(pin, Err(LlmError::Unsupported { .. })));
    }

    #[tokio::test]
    async fn a_pinned_route_wins_until_it_is_reset() {
        let router = router(StaticCredentials::new().key(ProviderId::Anthropic, "sk-ant-1"));
        let gpt = target(ProviderId::OpenAi, "gpt-5");

        let pinned = router.set(Route::Chat, Some(gpt.clone())).await.unwrap();
        let reset = router.set(Route::Chat, None).await.unwrap();

        assert_eq!((pinned.target, pinned.source), (gpt, RouteSource::Custom));
        assert_eq!(
            reset.target,
            target(ProviderId::Anthropic, CLAUDE_SMART_MODEL)
        );
        assert_eq!(reset.source, RouteSource::Default);
    }

    #[test]
    fn overrides_serialize_as_a_map_of_dotted_route_names() {
        let mut overrides = RouteOverrides::default();
        overrides.set(
            Route::Title,
            Some(target(ProviderId::Groq, "llama-3.3-70b-versatile")),
        );

        assert_eq!(
            serde_json::to_value(&overrides).unwrap(),
            serde_json::json!({ "title": { "provider": "groq", "model": "llama-3.3-70b-versatile" } })
        );
    }
}
