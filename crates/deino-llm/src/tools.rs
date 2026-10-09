//! Tool calling: typed tools and the agent loop behind `Gateway::with_tools`.
//!
//! A [`Tool`] declares a typed input (its JSON schema comes from the type)
//! and returns text for the model. [`Toolbox`] holds a set of tools and the
//! loop's step limit. The loop asks the model, runs every tool call it makes,
//! feeds the results back, and repeats until the model answers without
//! calling a tool, or fails with [`LlmError::ToolLoopExceeded`]. A tool's
//! failure (or an unknown tool, or invalid input) is returned to the model as
//! an error result, so it can correct itself; it does not end the loop.

use std::future::Future;
use std::pin::Pin;

use async_trait::async_trait;
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::gateway::Shape;
use crate::{
    Completion, Gateway, LlmError, Message, Part, Prompt, Role, Route, ToolCall, ToolDefinition,
    ToolResult, json_schema_for,
};

/// The loop's default step limit: model turns, not tool calls.
pub const DEFAULT_MAX_TOOL_STEPS: usize = 8;

/// A tool failed. The message goes to the model, never to the log.
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct ToolError(pub String);

/// A tool the model may call.
#[async_trait]
pub trait Tool: Send + Sync + 'static {
    /// The tool's input, parsed from the model's JSON.
    type Input: JsonSchema + DeserializeOwned + Send;

    /// A unique `snake_case` name.
    fn name(&self) -> &'static str;

    /// When and why the model should call it.
    fn description(&self) -> &'static str;

    /// Runs the tool and returns text for the model.
    async fn call(&self, input: Self::Input) -> Result<String, ToolError>;
}

type BoxedCall<'a> = Pin<Box<dyn Future<Output = Result<String, ToolError>> + Send + 'a>>;

/// [`Tool`] with its input erased, so tools of different types share a box.
trait ErasedTool: Send + Sync {
    fn definition(&self) -> ToolDefinition;
    fn call_json(&self, input: Value) -> BoxedCall<'_>;
}

impl<T: Tool> ErasedTool for T {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: self.name().to_owned(),
            description: self.description().to_owned(),
            input_schema: json_schema_for::<T::Input>(),
        }
    }

    fn call_json(&self, input: Value) -> BoxedCall<'_> {
        Box::pin(async move {
            let input = serde_json::from_value(input)
                .map_err(|error| ToolError(format!("invalid input: {error}")))?;
            self.call(input).await
        })
    }
}

/// The tools of one loop and its step limit.
pub struct Toolbox {
    tools: Vec<Box<dyn ErasedTool>>,
    max_steps: usize,
}

impl Default for Toolbox {
    fn default() -> Self {
        Self {
            tools: Vec::new(),
            max_steps: DEFAULT_MAX_TOOL_STEPS,
        }
    }
}

impl Toolbox {
    /// No tools, [`DEFAULT_MAX_TOOL_STEPS`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a tool.
    #[must_use]
    pub fn with(mut self, tool: impl Tool) -> Self {
        self.tools.push(Box::new(tool));
        self
    }

    /// Caps the loop at `steps` model turns.
    #[must_use]
    pub fn max_steps(mut self, steps: usize) -> Self {
        self.max_steps = steps;
        self
    }

    fn definitions(&self) -> Vec<ToolDefinition> {
        self.tools.iter().map(|tool| tool.definition()).collect()
    }

    /// Runs one call and wraps its outcome as the model's tool result.
    async fn run(&self, call: &ToolCall) -> ToolResult {
        let tool = self
            .tools
            .iter()
            .find(|tool| tool.definition().name == call.name);
        let outcome = match tool {
            Some(tool) => tool.call_json(call.input.clone()).await,
            None => Err(ToolError(format!("there is no tool named {}", call.name))),
        };
        let (content, is_error) = match outcome {
            Ok(content) => (content, false),
            Err(error) => (error.0, true),
        };
        ToolResult {
            call_id: call.id.clone(),
            content,
            is_error,
        }
    }
}

/// The result of a tool loop.
#[derive(Clone, Debug, PartialEq)]
pub struct ToolRun {
    /// The model's final answer (no tool calls).
    pub completion: Completion,
    /// Every tool call the model made, in order.
    pub calls: Vec<ToolCall>,
    /// How many model turns it took.
    pub steps: usize,
}

impl Gateway {
    /// Runs the tool loop for `route` (see the module docs).
    pub async fn with_tools(
        &self,
        route: Route,
        mut prompt: Prompt,
        toolbox: &Toolbox,
    ) -> Result<ToolRun, LlmError> {
        let shape = Shape {
            tools: toolbox.definitions(),
            ..Shape::default()
        };
        let mut calls = Vec::new();
        for step in 1..=toolbox.max_steps {
            let completion = self
                .complete_shaped(route, prompt.clone(), shape.clone())
                .await?;
            if completion.tool_calls.is_empty() {
                return Ok(ToolRun {
                    completion,
                    calls,
                    steps: step,
                });
            }
            log::debug!(target: "llm", "{route}: step {step} calls {} tools", completion.tool_calls.len());
            let results = run_all(toolbox, &completion.tool_calls).await;
            prompt.messages.push(assistant_turn(&completion));
            prompt.messages.push(Message {
                role: Role::User,
                parts: results,
            });
            calls.extend(completion.tool_calls);
        }
        Err(LlmError::ToolLoopExceeded {
            steps: toolbox.max_steps,
        })
    }
}

async fn run_all(toolbox: &Toolbox, calls: &[ToolCall]) -> Vec<Part> {
    let mut results = Vec::with_capacity(calls.len());
    for call in calls {
        results.push(Part::ToolResult(toolbox.run(call).await));
    }
    results
}

fn assistant_turn(completion: &Completion) -> Message {
    let text = (!completion.text.is_empty()).then(|| Part::Text(completion.text.clone()));
    let calls = completion.tool_calls.iter().cloned().map(Part::ToolUse);
    Message {
        role: Role::Assistant,
        parts: text.into_iter().chain(calls).collect(),
    }
}

#[cfg(test)]
mod tools_tests;
