//! `MockProvider`: scripted answers keyed by route, for every LLM test
//! (PARITY_PLAN §3.8). Behind the `test-support` feature.
//!
//! Script each route's turns in order; a turn is a text reply, a tool call,
//! raw events or an error. Calls made concurrently (NOT-02's map step) can
//! instead be answered from the request itself with
//! [`MockProvider::answer_with`], which does not depend on call order. [`mock_gateway`] builds a real [`Gateway`] whose
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

/// A scripted answer: its events, and whether the stream then hangs.
struct Scripted {
    events: Vec<CompletionEvent>,
    stalls: bool,
}

type Turn = Result<Scripted, LlmError>;

/// Answers a request with text, from what it asks.
type Responder = Arc<dyn Fn(&CompletionRequest) -> String + Send + Sync>;

/// Answers each route with its scripted turns, in order, then with its
/// responder, and records every request. A route with neither left fails
/// with [`LlmError::Unavailable`].
#[derive(Default)]
pub struct MockProvider {
    turns: Mutex<BTreeMap<Route, VecDeque<Turn>>>,
    responders: Mutex<BTreeMap<Route, Responder>>,
    requests: Mutex<Vec<CompletionRequest>>,
    context_window: Option<u32>,
}

impl MockProvider {
    /// No turns scripted.
    pub fn new() -> Self {
        Self::default()
    }

    /// Reports a context window of `tokens`, as a local model does.
    #[must_use]
    pub fn with_context_window(mut self, tokens: u32) -> Self {
        self.context_window = Some(tokens);
        self
    }

    /// Scripts a text answer for `route`.
    #[must_use]
    pub fn reply(self, route: Route, text: &str) -> Self {
        self.events(route, text_turn(text))
    }

    /// Scripts a text answer for `route` streamed in pieces of `size`
    /// characters, as a provider sends it.
    #[must_use]
    pub fn reply_in_chunks(self, route: Route, text: &str, size: usize) -> Self {
        let characters: Vec<char> = text.chars().collect();
        let mut events: Vec<CompletionEvent> = characters
            .chunks(size.max(1))
            .map(|piece| CompletionEvent::Text(piece.iter().collect()))
            .collect();
        events.push(CompletionEvent::Stop(StopReason::EndTurn));
        self.events(route, events)
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
        self.push(
            route,
            Ok(Scripted {
                events,
                stalls: false,
            }),
        )
    }

    /// Scripts a turn that streams `text` and then never ends, as a slow
    /// model mid-answer does (for stopping a call in flight).
    #[must_use]
    pub fn stall(self, route: Route, text: &str) -> Self {
        let events = vec![CompletionEvent::Text(text.to_owned())];
        self.push(
            route,
            Ok(Scripted {
                events,
                stalls: true,
            }),
        )
    }

    /// Answers every call on `route` that no scripted turn is left for with
    /// the text `respond` builds from the request.
    #[must_use]
    pub fn answer_with(
        self,
        route: Route,
        respond: impl Fn(&CompletionRequest) -> String + Send + Sync + 'static,
    ) -> Self {
        lock(&self.responders).insert(route, Arc::new(respond));
        self
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
        let scripted = lock(&self.turns)
            .get_mut(&route)
            .and_then(VecDeque::pop_front);
        let responder = lock(&self.responders).get(&route).cloned();
        let turn = scripted.or_else(|| {
            responder.map(|respond| {
                Ok(Scripted {
                    events: text_turn(&respond(&request)),
                    stalls: false,
                })
            })
        });
        lock(&self.requests).push(request);
        let Scripted { events, stalls } = turn.unwrap_or_else(|| {
            Err(LlmError::Unavailable {
                provider: ProviderId::BuiltIn,
                reason: format!("MockProvider has no turn scripted for {route}"),
            })
        })?;
        let events = stream::iter(events.into_iter().map(Ok));
        if stalls {
            return Ok(events.chain(stream::pending()).boxed());
        }
        Ok(events.boxed())
    }

    async fn context_window(&self, _model: &str) -> Option<u32> {
        self.context_window
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

/// A finished text answer, with usage.
fn text_turn(text: &str) -> Vec<CompletionEvent> {
    let output_tokens = u32::try_from(text.split_whitespace().count()).unwrap_or(u32::MAX);
    vec![
        CompletionEvent::Text(text.to_owned()),
        CompletionEvent::Usage(Usage {
            input_tokens: 100,
            output_tokens,
        }),
        CompletionEvent::Stop(StopReason::EndTurn),
    ]
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
