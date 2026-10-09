//! `MockProvider`: scripted answers keyed by route, for every LLM test
//! (PARITY_PLAN §3.8). Behind the `test-support` feature.
//!
//! Script each route's turns in order; a turn is a text reply, a tool call,
//! raw events or an error. [`mock_gateway`] builds a real [`Gateway`] whose
//! every provider is the mock, with an in-memory usage log and a fixed clock.

use std::collections::{BTreeMap, VecDeque};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_domain::FakeClock;
use futures_util::StreamExt;
use futures_util::stream;
use serde_json::Value;

use crate::{
    CompletionEvent, CompletionRequest, EventStream, FakeLocalModels, Gateway, GatewayParts,
    LlmError, LlmProvider, MemoryRouteStore, MemoryUsage, ProviderId, Providers, Route, Router,
    StaticCredentials, StopReason, ToolCall, Usage,
};

/// The built-in model name routes resolve to in a [`mock_gateway`].
pub const MOCK_LOCAL_MODEL: &str = "mock-local";

type Turn = Result<Vec<CompletionEvent>, LlmError>;

/// Answers each route with its scripted turns, in order, and records every
/// request. A route with no turn left fails with [`LlmError::Unavailable`].
#[derive(Default)]
pub struct MockProvider {
    turns: Mutex<BTreeMap<Route, VecDeque<Turn>>>,
    requests: Mutex<Vec<CompletionRequest>>,
}

impl MockProvider {
    /// No turns scripted.
    pub fn new() -> Self {
        Self::default()
    }

    /// Scripts a text answer for `route`.
    #[must_use]
    pub fn reply(self, route: Route, text: &str) -> Self {
        let output_tokens = u32::try_from(text.split_whitespace().count()).unwrap_or(u32::MAX);
        self.events(
            route,
            vec![
                CompletionEvent::Text(text.to_owned()),
                CompletionEvent::Usage(Usage {
                    input_tokens: 100,
                    output_tokens,
                }),
                CompletionEvent::Stop(StopReason::EndTurn),
            ],
        )
    }

    /// Scripts a turn in which the model calls tool `name` with `input`.
    #[must_use]
    pub fn call_tool(self, route: Route, name: &str, input: Value) -> Self {
        let id = format!("call-{}", self.requests_scripted(route) + 1);
        let call = ToolCall {
            id,
            name: name.to_owned(),
            input,
        };
        self.events(
            route,
            vec![
                CompletionEvent::ToolUse(call),
                CompletionEvent::Stop(StopReason::ToolUse),
            ],
        )
    }

    /// Scripts a turn that yields exactly `events`.
    #[must_use]
    pub fn events(self, route: Route, events: Vec<CompletionEvent>) -> Self {
        self.push(route, Ok(events))
    }

    /// Scripts a turn that fails before streaming.
    #[must_use]
    pub fn fail(self, route: Route, error: LlmError) -> Self {
        self.push(route, Err(error))
    }

    /// Every request received, oldest first.
    pub fn requests(&self) -> Vec<CompletionRequest> {
        lock(&self.requests).clone()
    }

    fn push(self, route: Route, turn: Turn) -> Self {
        lock(&self.turns).entry(route).or_default().push_back(turn);
        self
    }

    fn requests_scripted(&self, route: Route) -> usize {
        lock(&self.turns).get(&route).map_or(0, VecDeque::len)
    }
}

#[async_trait]
impl LlmProvider for MockProvider {
    async fn stream(&self, request: CompletionRequest) -> Result<EventStream, LlmError> {
        let route = request.route;
        lock(&self.requests).push(request);
        let turn = lock(&self.turns)
            .get_mut(&route)
            .and_then(VecDeque::pop_front);
        let events = turn.unwrap_or_else(|| {
            Err(LlmError::Unavailable {
                provider: ProviderId::BuiltIn,
                reason: format!("MockProvider has no turn scripted for {route}"),
            })
        })?;
        Ok(stream::iter(events.into_iter().map(Ok)).boxed())
    }
}

/// A gateway over a mock, and what tests inspect.
pub struct MockGateway {
    /// The gateway; every provider is the mock.
    pub gateway: Gateway,
    /// The usage records it wrote.
    pub usage: Arc<MemoryUsage>,
    /// The clock it reads, fixed until a test advances it.
    pub clock: Arc<FakeClock>,
    /// The mock behind every provider.
    pub mock: Arc<MockProvider>,
}

/// A real [`Gateway`] whose every provider is `mock`. With no keys set,
/// every text route resolves to the built-in [`MOCK_LOCAL_MODEL`].
pub fn mock_gateway(mock: Arc<MockProvider>) -> MockGateway {
    let start: DateTime<Utc> = DateTime::from_timestamp(1_791_536_400, 0).unwrap_or_default();
    let clock = Arc::new(FakeClock::new(start));
    let usage = Arc::new(MemoryUsage::default());
    let providers = ProviderId::ALL
        .into_iter()
        .fold(Providers::new(), |providers, id| {
            providers.with(id, mock.clone())
        });
    let local = Arc::new(FakeLocalModels::new().with_models(&[MOCK_LOCAL_MODEL]));
    let router = Router::new(
        Arc::new(MemoryRouteStore::default()),
        Arc::new(StaticCredentials::new()),
        local.clone(),
    );
    let gateway = Gateway::new(GatewayParts {
        providers,
        router,
        local,
        usage: usage.clone(),
        clock: clock.clone(),
    });
    MockGateway {
        gateway,
        usage,
        clock,
        mock,
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
