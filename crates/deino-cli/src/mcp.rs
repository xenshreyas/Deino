//! `deino mcp`: the library as an MCP server over stdio (DEV-02, ADR 0033),
//! for Claude Code (`claude mcp add deino -- deino mcp`), Claude Desktop
//! and any other MCP client.
//!
//! The server is read-only: every tool is a [`Request`](crate::request::Request)
//! from [`registry`]. It opens the database on the first call and keeps it
//! open; when that fails (no database yet, a schema the binary does not
//! know) each call answers with the reason, and the next call tries again,
//! so a client never has to restart it. Nothing is ever written to stdout
//! but the protocol, and nothing is logged.

pub mod registry;

use std::path::PathBuf;
use std::sync::Arc;

use rmcp::model::{
    CallToolRequestParams, CallToolResponse, Implementation, ListToolsResult,
    PaginatedRequestParams, ServerCapabilities, ServerConfig,
};
use rmcp::service::RequestContext;
use rmcp::{ErrorData, RoleServer, ServerHandler, ServiceExt};
use tokio::sync::OnceCell;

use self::registry::{ToolEntry, failure, tools};
use crate::{CliError, Opened};

/// What the server tells a client about itself.
const INSTRUCTIONS: &str = "Deino is the user's private meeting notepad. These tools read \
    their recorded meetings (titles, dates, attendees, tags, folders), their own notes and the \
    AI notes, action items, transcripts and calendar, straight from the Deino database on this \
    Mac; nothing can be changed. Start with list_meetings (recent or filtered) or search (what \
    was discussed), then read_meeting for notes and action items and get_transcript for exact \
    words. Times are the user's local time. Cite meetings by title and date, and link moments \
    as deino://meeting/<id>?t=<ms>.";

/// The MCP server over one database.
#[derive(Clone)]
pub struct DeinoServer {
    tools: Arc<Vec<Arc<dyn ToolEntry>>>,
    database: PathBuf,
    opened: Arc<OnceCell<Opened>>,
}

impl DeinoServer {
    /// A server that reads the database at `database`.
    pub fn new(database: PathBuf) -> Self {
        Self {
            tools: Arc::new(tools()),
            database,
            opened: Arc::new(OnceCell::new()),
        }
    }

    /// The open database, opening it on first use.
    async fn opened(&self) -> Result<&Opened, CliError> {
        self.opened
            .get_or_try_init(|| Opened::read_only(&self.database))
            .await
    }
}

impl ServerHandler for DeinoServer {
    fn get_info(&self) -> ServerConfig {
        let identity = Implementation::new("deino", env!("CARGO_PKG_VERSION")).with_title("Deino");
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(identity)
            .with_instructions(INSTRUCTIONS)
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        let specs: Result<Vec<_>, CliError> = self.tools.iter().map(|tool| tool.spec()).collect();
        let specs = specs.map_err(|error| ErrorData::internal_error(error.to_string(), None))?;
        Ok(ListToolsResult::with_all_items(specs))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        let Some(tool) = self.tools.iter().find(|tool| tool.name() == request.name) else {
            let message = format!("there is no tool named {}", request.name);
            return Err(ErrorData::invalid_params(message, None));
        };
        let arguments = request.arguments.unwrap_or_default();
        let result = match self.opened().await {
            Ok(opened) => tool.call(&opened.library, arguments).await,
            Err(error) => failure(&error),
        };
        Ok(result.into())
    }
}

/// Serves the database at `database` on stdin and stdout until the client
/// disconnects.
pub async fn serve(database: PathBuf) -> Result<(), CliError> {
    let running = DeinoServer::new(database)
        .serve(rmcp::transport::stdio())
        .await
        .map_err(|error| CliError::Mcp(error.to_string()))?;
    running
        .waiting()
        .await
        .map_err(|error| CliError::Mcp(error.to_string()))?;
    Ok(())
}
