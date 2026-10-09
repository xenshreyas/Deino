//! Deino's LLM gateway (PARITY_PLAN §4.3, ADR 0013).
//!
//! Every LLM call in Deino goes through [`Gateway`] and names a [`Route`]
//! (what the call is for). Three layers, the zed pattern reimplemented:
//!
//! 1. **Wire types** per provider (`providers/<provider>/wire.rs`): the
//!    provider's JSON, nothing else.
//! 2. **Adapters** implementing [`LlmProvider`] (one required method,
//!    `stream`): [`AnthropicProvider`], [`OpenAiCompatProvider`] (OpenAI,
//!    Groq, OpenRouter, custom), [`OllamaProvider`] and [`BuiltInProvider`]
//!    (the llama-helper sidecar). Each streams [`CompletionEvent`]s.
//! 3. **The gateway**: route resolution ([`Router`]), text, streaming,
//!    [`Gateway::structured`] (typed JSON with one repair retry),
//!    [`Gateway::with_tools`] (a tool loop over typed [`Tool`]s),
//!    [`Gateway::embed`], and usage metering into `llm_calls`.
//!
//! The crate reaches the outside world only through ports the app wires:
//! [`HttpTransport`] (a `deino-net` scoped client per provider), [`CredentialSource`] and
//! [`EndpointSource`] (until FND-06's Keychain secrets and settings),
//! [`RouteStore`], [`LocalModels`] (the sidecar) and [`UsageRecorder`]. It does
//! not depend on Tauri, and nothing it logs contains prompt or answer content.
//!
//! Built-in prompts are askama templates ([`prompts`]); user templates are
//! minijinja with an allowlist ([`UserTemplate`]). Tests script answers with
//! `MockProvider` (feature `test-support`).

pub mod admin;
mod catalog;
mod credentials;
mod error;
mod event;
mod gateway;
mod http;
#[cfg(any(test, feature = "test-support"))]
mod mock;
pub mod prompts;
mod provider;
mod provider_id;
pub mod providers;
mod request;
mod route;
mod router;
mod sse;
mod structured;
mod tools;
mod usage;
mod user_templates;

pub use admin::{AdminParts, LlmAdmin, ProviderStatus, RouteTestReport, TextSink};
pub use catalog::{DraftConnection, ModelCatalog, ModelInfo};
pub use credentials::{
    ApiKey, CredentialError, CredentialSource, Endpoint, EndpointSource, SamplingOverrides,
    StaticCredentials,
};
pub use error::{LlmError, LlmErrorKind};
pub use event::{Completion, CompletionEvent, EventStream, StopReason, Usage};
pub use gateway::{DEFAULT_MAX_TOKENS, Gateway, GatewayParts};
#[cfg(any(test, feature = "test-support"))]
pub use http::FakeHttp;
pub use http::{
    BodyStream, COMPLETION_TIMEOUT, HttpError, HttpRequest, HttpResponse, HttpTransport,
    METADATA_TIMEOUT, Method,
};
#[cfg(any(test, feature = "test-support"))]
pub use mock::{MOCK_LOCAL_MODEL, MockGateway, MockProvider, mock_gateway};
pub use provider::{LlmProvider, Providers};
pub use provider_id::ProviderId;
#[cfg(any(test, feature = "test-support"))]
pub use providers::builtin::FakeLocalModels;
pub use providers::{
    AnthropicProvider, BuiltInProvider, EmbedReply, Flavor, HelperCompletion, HelperEvent,
    HelperEventStream, HelperRequest, HelperSampling, HelperStop, LocalGeneration, LocalModels,
    OllamaProvider, OpenAiCompatProvider, PullProgress,
};
pub use request::{
    CompletionRequest, JsonOutput, Message, OutputFormat, Part, Prompt, Role, Target, ToolCall,
    ToolDefinition, ToolResult,
};
pub use route::{Route, RouteTier};
pub use router::{
    CLAUDE_FAST_MODEL, CLAUDE_SMART_MODEL, LOCAL_EMBEDDING_MODEL, MemoryRouteStore, ResolvedRoute,
    RouteOverrides, RouteSource, RouteStore, RouteStoreError, Router,
};
pub use sse::{SseEvent, SseParser};
pub use structured::json_schema_for;
pub use tools::{DEFAULT_MAX_TOOL_STEPS, Tool, ToolError, ToolRun, Toolbox};
pub use usage::{
    CallOutcome, LlmCall, LlmCallRepo, MemoryUsage, SqliteLlmCallRepo, UsageChannel, UsageRecorder,
    UsageWriter,
};
pub use user_templates::{TemplateError, UserTemplate};
