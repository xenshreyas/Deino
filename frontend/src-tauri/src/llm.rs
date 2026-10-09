//! The LLM gateway's wiring: the app's adapters for `deino-llm`'s ports and
//! the services built from them (part of the composition root).
//!
//! | Port | Adapter |
//! |---|---|
//! | `HttpTransport` | [`http::ReqwestTransport`] (until FND-12's `deino-net`) |
//! | `CredentialSource`, `EndpointSource`, `RouteStore` | `deino_core::settings::LlmAccess` (Keychain + settings) |
//! | `LocalModels` | [`local::SidecarLocalModels`] (the llama-helper sidecar) |
//! | `UsageRecorder` | `UsageChannel` → `UsageWriter` → `SqliteLlmCallRepo` (`llm_calls`) |

mod http;
mod local;
#[cfg(test)]
mod smoke_tests;

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use deino_core::settings::LlmAccess;
use deino_core::summaries::SummaryGenerator;
use deino_db::Database;
use deino_domain::Clock;
use deino_llm::{
    AdminParts, AnthropicProvider, BuiltInProvider, Flavor, Gateway, GatewayParts, HttpTransport,
    LlmAdmin, LlmProvider, LocalModels, ModelCatalog, OllamaProvider, OpenAiCompatProvider,
    ProviderId, Providers, Router, SqliteLlmCallRepo, UsageChannel, UsageWriter,
};

/// What the LLM services are built from.
pub(crate) struct LlmPorts {
    /// Keys, endpoints and route choices.
    pub access: Arc<LlmAccess>,
    /// Where `llm_calls` lives.
    pub database: Database,
    /// `<app_data_dir>`, for the built-in models.
    pub app_data_dir: PathBuf,
    /// Time for latencies and usage records.
    pub clock: Arc<dyn Clock>,
}

/// The LLM services the app manages, and the usage writer to run.
pub(crate) struct LlmServices {
    /// The gateway every LLM call goes through.
    pub gateway: Arc<Gateway>,
    /// Settings → AI.
    pub admin: Arc<LlmAdmin>,
    /// The legacy template summaries.
    pub summaries: Arc<SummaryGenerator>,
    /// Drains usage records into `llm_calls`; run it once.
    pub usage_writer: UsageWriter,
}

/// Builds the gateway, its providers and the services over it.
pub(crate) fn build(ports: LlmPorts) -> LlmServices {
    let http: Arc<dyn HttpTransport> = Arc::new(http::ReqwestTransport::new());
    let local: Arc<dyn LocalModels> = Arc::new(local::SidecarLocalModels::new(ports.app_data_dir));
    let ollama = Arc::new(OllamaProvider::new(
        http.clone(),
        ports.access.clone(),
        ports.access.clone(),
    ));
    let (providers, catalogs) = registry(adapters(&http, &ports.access, &local), &ollama);
    let (usage, usage_writer) = UsageChannel::new(Arc::new(SqliteLlmCallRepo::new(ports.database)));
    let gateway = Arc::new(Gateway::new(GatewayParts {
        providers,
        router: Router::new(ports.access.clone(), ports.access.clone(), local.clone()),
        local,
        usage: Arc::new(usage),
        clock: ports.clock.clone(),
    }));
    let admin = Arc::new(LlmAdmin::new(AdminParts {
        gateway: gateway.clone(),
        catalogs,
        credentials: ports.access.clone(),
        endpoints: ports.access,
        ollama,
        clock: ports.clock,
    }));
    LlmServices {
        summaries: Arc::new(SummaryGenerator::new(gateway.clone())),
        gateway,
        admin,
        usage_writer,
    }
}

/// The gateway's providers and the admin's catalogs, Ollama included.
fn registry(
    adapters: BTreeMap<ProviderId, Adapter>,
    ollama: &Arc<OllamaProvider>,
) -> (Providers, BTreeMap<ProviderId, Arc<dyn ModelCatalog>>) {
    let providers = adapters
        .iter()
        .fold(Providers::new(), |providers, (id, adapter)| {
            providers.with(*id, adapter.provider.clone())
        })
        .with(ProviderId::Ollama, ollama.clone());
    let mut catalogs: BTreeMap<ProviderId, Arc<dyn ModelCatalog>> = adapters
        .into_iter()
        .map(|(id, adapter)| (id, adapter.catalog))
        .collect();
    catalogs.insert(ProviderId::Ollama, ollama.clone());
    (providers, catalogs)
}

/// One adapter, seen as a provider and as a model catalog.
struct Adapter {
    provider: Arc<dyn LlmProvider>,
    catalog: Arc<dyn ModelCatalog>,
}

impl Adapter {
    fn of<A: LlmProvider + ModelCatalog + 'static>(adapter: A) -> Self {
        let adapter = Arc::new(adapter);
        Self {
            provider: adapter.clone(),
            catalog: adapter,
        }
    }
}

/// Every adapter but Ollama's, keyed by provider.
fn adapters(
    http: &Arc<dyn HttpTransport>,
    access: &Arc<LlmAccess>,
    local: &Arc<dyn LocalModels>,
) -> BTreeMap<ProviderId, Adapter> {
    let mut adapters = BTreeMap::from([
        (
            ProviderId::Anthropic,
            Adapter::of(AnthropicProvider::new(http.clone(), access.clone())),
        ),
        (
            ProviderId::BuiltIn,
            Adapter::of(BuiltInProvider::new(local.clone())),
        ),
    ]);
    for flavor in [
        Flavor::OpenAi,
        Flavor::Groq,
        Flavor::OpenRouter,
        Flavor::Custom,
    ] {
        let adapter =
            OpenAiCompatProvider::new(flavor, http.clone(), access.clone(), access.clone());
        adapters.insert(flavor.provider(), Adapter::of(adapter));
    }
    adapters
}
