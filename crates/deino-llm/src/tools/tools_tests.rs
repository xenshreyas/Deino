//! The tool loop over `MockProvider` with scripted tool calls.

use std::sync::{Arc, Mutex};

use serde::Deserialize;
use serde_json::json;

use super::*;
use crate::{MockProvider, OutputFormat, mock_gateway};

/// Looks meetings up by a query (the AI-02 chat tool, in miniature).
#[derive(Default)]
struct SearchMeetings {
    queries: Mutex<Vec<String>>,
}

#[derive(Deserialize, JsonSchema)]
struct SearchInput {
    /// What to search for.
    query: String,
}

#[async_trait]
impl Tool for SearchMeetings {
    type Input = SearchInput;

    fn name(&self) -> &'static str {
        "search_meetings"
    }

    fn description(&self) -> &'static str {
        "Search past meetings by topic."
    }

    async fn call(&self, input: SearchInput) -> Result<String, ToolError> {
        self.queries.lock().unwrap().push(input.query.clone());
        if input.query == "fail" {
            return Err(ToolError("the index is rebuilding".into()));
        }
        Ok(format!("2 meetings about {}", input.query))
    }
}

/// A tool that shares state with the test through an `Arc`.
struct SharedSearch(Arc<SearchMeetings>);

#[async_trait]
impl Tool for SharedSearch {
    type Input = SearchInput;

    fn name(&self) -> &'static str {
        self.0.name()
    }

    fn description(&self) -> &'static str {
        self.0.description()
    }

    async fn call(&self, input: SearchInput) -> Result<String, ToolError> {
        self.0.call(input).await
    }
}

fn toolbox() -> (Toolbox, Arc<SearchMeetings>) {
    let search = Arc::new(SearchMeetings::default());
    (Toolbox::new().with(SharedSearch(search.clone())), search)
}

#[tokio::test]
async fn the_loop_runs_tool_calls_and_returns_the_final_answer() {
    let mock = Arc::new(
        MockProvider::new()
            .call_tool(
                Route::Chat,
                "search_meetings",
                json!({ "query": "pricing" }),
            )
            .reply(Route::Chat, "You discussed pricing twice."),
    );
    let harness = mock_gateway(mock.clone());
    let (toolbox, search) = toolbox();

    let run = harness
        .gateway
        .with_tools(Route::Chat, Prompt::new("What about pricing?"), &toolbox)
        .await
        .unwrap();

    assert_eq!(run.completion.text, "You discussed pricing twice.");
    assert_eq!(run.steps, 2);
    assert_eq!(run.calls.len(), 1);
    assert_eq!(*search.queries.lock().unwrap(), ["pricing"]);
    let second = &mock.requests()[1];
    assert_eq!(second.tools[0].name, "search_meetings");
    assert_eq!(
        second.tools[0].input_schema["properties"]["query"]["type"],
        "string"
    );
    assert_eq!(second.output, OutputFormat::Text);
    let Part::ToolResult(result) = &second.messages[2].parts[0] else {
        panic!("the third message answers the tool call");
    };
    assert_eq!(result.content, "2 meetings about pricing");
    assert!(!result.is_error);
}

#[tokio::test]
async fn tool_failures_unknown_tools_and_bad_input_go_back_to_the_model() {
    let mock = Arc::new(
        MockProvider::new()
            .call_tool(Route::Chat, "search_meetings", json!({ "query": "fail" }))
            .call_tool(Route::Chat, "delete_everything", json!({}))
            .call_tool(Route::Chat, "search_meetings", json!({ "q": 1 }))
            .reply(Route::Chat, "I could not search."),
    );
    let harness = mock_gateway(mock.clone());
    let (toolbox, _) = toolbox();

    let run = harness
        .gateway
        .with_tools(Route::Chat, Prompt::new("x"), &toolbox)
        .await
        .unwrap();

    assert_eq!(run.steps, 4);
    let last = mock.requests().pop().unwrap();
    let errors: Vec<String> = last
        .messages
        .iter()
        .flat_map(|message| &message.parts)
        .filter_map(|part| match part {
            Part::ToolResult(result) if result.is_error => Some(result.content.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(errors[0], "the index is rebuilding");
    assert_eq!(errors[1], "there is no tool named delete_everything");
    assert!(
        errors[2].starts_with("invalid input: missing field `query`"),
        "{}",
        errors[2]
    );
}

#[tokio::test]
async fn the_loop_stops_at_its_step_limit() {
    let mock = Arc::new(
        MockProvider::new()
            .call_tool(Route::Chat, "search_meetings", json!({ "query": "a" }))
            .call_tool(Route::Chat, "search_meetings", json!({ "query": "b" })),
    );
    let harness = mock_gateway(mock);
    let (toolbox, _) = toolbox();

    let error = harness
        .gateway
        .with_tools(Route::Chat, Prompt::new("x"), &toolbox.max_steps(2))
        .await
        .unwrap_err();

    assert!(matches!(error, LlmError::ToolLoopExceeded { steps: 2 }));
}

#[tokio::test]
async fn the_assistant_turn_keeps_the_tool_calls_it_answered() {
    let mock = Arc::new(
        MockProvider::new()
            .call_tool(Route::Chat, "search_meetings", json!({ "query": "a" }))
            .reply(Route::Chat, "done"),
    );
    let harness = mock_gateway(mock.clone());
    let (toolbox, _) = toolbox();

    harness
        .gateway
        .with_tools(Route::Chat, Prompt::new("x"), &toolbox)
        .await
        .unwrap();

    let assistant = &mock.requests()[1].messages[1];
    assert_eq!(assistant.role, Role::Assistant);
    assert!(matches!(&assistant.parts[0], Part::ToolUse(call) if call.name == "search_meetings"));
}
