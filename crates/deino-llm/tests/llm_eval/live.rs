//! Live trials: a real provider, with keys from the environment.
//!
//! The only `reqwest` use in this crate, and only in this test binary, until
//! FND-12's `deino-net` provides a transport tests can share.

#![allow(
    clippy::disallowed_types,
    reason = "the eval harness's own HTTP transport until FND-12 (deino-net)"
)]

use std::sync::Arc;

use async_trait::async_trait;
use deino_domain::SystemClock;
use deino_llm::{
    AnthropicProvider, FakeLocalModels, Flavor, Gateway, GatewayParts, HttpError, HttpRequest,
    HttpResponse, HttpTransport, MemoryRouteStore, MemoryUsage, Method, OpenAiCompatProvider,
    ProviderId, Providers, Route, Router, StaticCredentials, Target,
};
use futures_util::StreamExt;
use libtest_mimic::Failed;

use crate::cases::Case;
use crate::expectations;

/// Runs `case` against a real provider `DEINO_EVAL_SAMPLES` times.
pub fn run(case: &Case) -> Result<(), Failed> {
    let samples = samples();
    let runtime =
        tokio::runtime::Runtime::new().map_err(|error| Failed::from(error.to_string()))?;
    let gateway = gateway();
    let mut prompt = case.prompt.clone();
    prompt.target = target();
    let mut failures = Vec::new();
    for _ in 0..samples {
        let completion = runtime
            .block_on(gateway.complete(case.route, prompt.clone()))
            .map_err(|error| Failed::from(error.to_string()))?;
        if let Err(reason) = expectations::check(&case.expectations, &completion.text) {
            failures.push(reason);
        }
    }
    let passed = samples - failures.len();
    #[allow(clippy::cast_precision_loss, reason = "sample counts are tiny")]
    let rate = passed as f64 / samples as f64;
    if rate >= case.required_pass_rate {
        Ok(())
    } else {
        Err(format!("{passed}/{samples} samples passed: {failures:?}").into())
    }
}

/// How many times each live case runs: `DEINO_EVAL_SAMPLES`, at least 1.
pub fn samples() -> usize {
    env("DEINO_EVAL_SAMPLES")
        .and_then(|value| value.parse().ok())
        .unwrap_or(1)
        .max(1)
}

/// The provider and model `DEINO_EVAL_TARGET` names, if any.
pub fn target() -> Option<Target> {
    env("DEINO_EVAL_TARGET").and_then(|target| parse_target(&target))
}

fn env(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
}

/// `provider:model`, for example `openai:gpt-5-mini`.
fn parse_target(target: &str) -> Option<Target> {
    let (provider, model) = target.split_once(':')?;
    let provider = serde_json::from_value(serde_json::Value::String(provider.to_owned())).ok()?;
    Some(Target {
        provider,
        model: model.to_owned(),
    })
}

/// A gateway over the real Anthropic and OpenAI providers, with keys from
/// `ANTHROPIC_API_KEY` and `OPENAI_API_KEY`.
pub fn gateway() -> Gateway {
    let (credentials, providers) = providers();
    let local = Arc::new(FakeLocalModels::new());
    let router = Router::new(
        Arc::new(MemoryRouteStore::default()),
        credentials,
        local.clone(),
    );
    assemble(providers, router, local)
}

/// A gateway whose `route` goes to `DEINO_EVAL_TARGET` when it is set, for
/// trials whose prompts the code under test builds (`notes.enhance`).
pub fn gateway_for(route: Route) -> Result<Gateway, Failed> {
    let (credentials, providers) = providers();
    let local = Arc::new(FakeLocalModels::new());
    let router = Router::new(
        Arc::new(MemoryRouteStore::default()),
        credentials,
        local.clone(),
    );
    if let Some(target) = env("DEINO_EVAL_TARGET").and_then(|target| parse_target(&target)) {
        let runtime = tokio::runtime::Runtime::new().map_err(|error| error.to_string())?;
        runtime
            .block_on(router.set(route, Some(target)))
            .map_err(|error| error.to_string())?;
    }
    Ok(assemble(providers, router, local))
}

/// The keys from the environment, and the hosted providers over `reqwest`.
fn providers() -> (Arc<StaticCredentials>, Providers) {
    let mut credentials = StaticCredentials::new();
    for (provider, variable) in [
        (ProviderId::Anthropic, "ANTHROPIC_API_KEY"),
        (ProviderId::OpenAi, "OPENAI_API_KEY"),
    ] {
        credentials = credentials.key(provider, &env(variable).unwrap_or_default());
    }
    let credentials = Arc::new(credentials);
    let http: Arc<dyn HttpTransport> = Arc::new(Reqwest(reqwest::Client::new()));
    let openai = OpenAiCompatProvider::new(
        Flavor::OpenAi,
        http.clone(),
        credentials.clone(),
        credentials.clone(),
    );
    let providers = Providers::new()
        .with(
            ProviderId::Anthropic,
            Arc::new(AnthropicProvider::new(http, credentials.clone())),
        )
        .with(ProviderId::OpenAi, Arc::new(openai));
    (credentials, providers)
}

fn assemble(providers: Providers, router: Router, local: Arc<FakeLocalModels>) -> Gateway {
    Gateway::new(GatewayParts {
        providers,
        router,
        local,
        usage: Arc::new(MemoryUsage::default()),
        clock: Arc::new(SystemClock),
    })
}

struct Reqwest(reqwest::Client);

#[async_trait]
impl HttpTransport for Reqwest {
    async fn send(&self, request: HttpRequest) -> Result<HttpResponse, HttpError> {
        let method = match request.method {
            Method::Get => reqwest::Method::GET,
            Method::Post => reqwest::Method::POST,
            Method::Delete => reqwest::Method::DELETE,
        };
        let mut builder = self
            .0
            .request(method, &request.url)
            .timeout(request.timeout);
        for (name, value) in request.headers {
            builder = builder.header(name, value);
        }
        if let Some(body) = request.body {
            builder = builder.body(body);
        }
        let response = builder
            .send()
            .await
            .map_err(|error| HttpError::Transport(error.without_url().to_string()))?;
        let status = response.status().as_u16();
        let body = response
            .bytes_stream()
            .map(|chunk| {
                chunk
                    .map(|bytes| bytes.to_vec())
                    .map_err(|error| HttpError::Transport(error.to_string()))
            })
            .boxed();
        Ok(HttpResponse {
            status,
            headers: Vec::new(),
            body,
        })
    }
}
