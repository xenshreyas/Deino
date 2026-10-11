//! The MCP tools: one [`ToolEntry`] per [`Request`], listed in [`tools`].
//!
//! Adding a tool is a `Request` implementation and one line in [`tools`];
//! the server, the CLI and the schemas need no other change.

use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::Arc;

use deino_core::query::Library;
use rmcp::handler::server::common::{schema_for_input, schema_for_output};
use rmcp::model::{CallToolResult, ContentBlock, JsonObject, Tool, ToolAnnotations};
use serde_json::{Value, json};

use crate::CliError;
use crate::request::{
    GetTranscript, ListActionItems, ListCalendarEvents, ListCompanies, ListFolders, ListMeetings,
    ListPeople, ListTags, Markdown, ReadMeetings, Request, SearchLibrary,
};

/// The argument every tool accepts besides its own: how to write the text.
const FORMAT: &str = "format";

/// A future a tool call returns.
pub type Call<'a> = Pin<Box<dyn Future<Output = CallToolResult> + Send + 'a>>;

/// One MCP tool.
pub trait ToolEntry: Send + Sync {
    /// Its name.
    fn name(&self) -> &'static str;
    /// Its definition for `tools/list`.
    fn spec(&self) -> Result<Tool, CliError>;
    /// Runs it on a call's `arguments`. Failures come back as tool errors
    /// the model can read, never as protocol errors.
    fn call<'a>(&'a self, library: &'a Library, arguments: JsonObject) -> Call<'a>;
}

/// Every tool, in the order `tools/list` shows them.
pub fn tools() -> Vec<Arc<dyn ToolEntry>> {
    vec![
        entry::<ListMeetings>(),
        entry::<ReadMeetings>(),
        entry::<GetTranscript>(),
        entry::<SearchLibrary>(),
        entry::<ListActionItems>(),
        entry::<ListPeople>(),
        entry::<ListCompanies>(),
        entry::<ListTags>(),
        entry::<ListFolders>(),
        entry::<ListCalendarEvents>(),
    ]
}

fn entry<R: Request>() -> Arc<dyn ToolEntry> {
    Arc::new(Entry::<R>(PhantomData))
}

/// The tool of request `R`.
struct Entry<R>(PhantomData<fn() -> R>);

impl<R: Request> ToolEntry for Entry<R> {
    fn name(&self) -> &'static str {
        R::NAME
    }

    fn spec(&self) -> Result<Tool, CliError> {
        let mut input = (*schema_for_input::<R>().map_err(CliError::Invalid)?).clone();
        add_format(&mut input);
        Ok(Tool::new(R::NAME, R::DESCRIPTION, Arc::new(input))
            .with_title(R::TITLE)
            .with_raw_output_schema(schema_for_output::<R::Output>())
            .with_annotations(
                ToolAnnotations::with_title(R::TITLE)
                    .read_only(true)
                    .destructive(false)
                    .idempotent(true)
                    .open_world(false),
            ))
    }

    fn call<'a>(&'a self, library: &'a Library, arguments: JsonObject) -> Call<'a> {
        Box::pin(async move {
            match answer::<R>(library, arguments).await {
                Ok(result) => result,
                Err(error) => failure(&error),
            }
        })
    }
}

/// How a tool writes its text content.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Format {
    Markdown,
    Json,
}

async fn answer<R: Request>(
    library: &Library,
    mut arguments: JsonObject,
) -> Result<CallToolResult, CliError> {
    let format = match arguments.remove(FORMAT) {
        None => Format::Markdown,
        Some(Value::String(text)) if text == "markdown" => Format::Markdown,
        Some(Value::String(text)) if text == "json" => Format::Json,
        Some(_) => {
            return Err(CliError::Invalid(
                "format must be \"markdown\" or \"json\"".into(),
            ));
        }
    };
    let request: R = serde_json::from_value(Value::Object(arguments)).map_err(|error| {
        CliError::Invalid(format!("invalid arguments for {}: {error}", R::NAME))
    })?;
    let output = request.run(library).await?;
    let structured = serde_json::to_value(&output)?;
    let text = match format {
        Format::Markdown => output.markdown(),
        Format::Json => serde_json::to_string_pretty(&structured)?,
    };
    let mut result = CallToolResult::success(vec![ContentBlock::text(text)]);
    result.structured_content = Some(structured);
    Ok(result)
}

/// A tool error the model can read: `{"error": {"code", "message"}}`.
pub fn failure(error: &CliError) -> CallToolResult {
    let body = json!({ "error": error.body() });
    CallToolResult::error(vec![ContentBlock::text(body.to_string())])
}

/// Adds the shared `format` argument to a tool's input schema.
fn add_format(schema: &mut JsonObject) {
    let property = json!({
        "type": "string",
        "enum": ["markdown", "json"],
        "description": "How to write the text result: compact Markdown (default) or JSON.",
    });
    if let Some(Value::Object(properties)) = schema.get_mut("properties") {
        properties.insert(FORMAT.to_owned(), property);
    } else {
        schema.insert("properties".to_owned(), json!({ FORMAT: property }));
    }
}
