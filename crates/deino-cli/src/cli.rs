//! The command line: [`Cli`] (clap) and [`execute`], which runs one command
//! and returns what to print.

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};
use serde::Serialize;

use crate::export::ExportArgs;
use crate::request::{
    GetTranscript, ListActionItems, ListCalendarEvents, ListCompanies, ListFolders, ListMeetings,
    ListPeople, ListTags, Markdown, ReadMeetings, Request, SearchLibrary,
};
use crate::status::status;
use crate::{CliError, Opened, db_path, mcp};

/// Read your Deino meetings, notes and transcripts from the terminal, and
/// serve them to Claude over MCP. The database is only ever read.
#[derive(Debug, Parser)]
#[command(name = "deino", version, max_term_width = 100)]
pub struct Cli {
    /// The database to read (default: the Deino app's own).
    #[arg(long, global = true, env = db_path::DB_ENV, value_name = "PATH")]
    pub db: Option<PathBuf>,
    /// Print JSON instead of Markdown.
    #[arg(long, global = true)]
    pub json: bool,
    /// What to do.
    #[command(subcommand)]
    pub command: Command,
}

/// The subcommands.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// List meetings, newest first (`deino meetings read <ids…>` reads them in full).
    Meetings(MeetingsArgs),
    /// Read transcripts.
    #[command(subcommand)]
    Transcripts(TranscriptsCommand),
    /// Search titles, notes, AI notes, action items and transcripts.
    Search(SearchLibrary),
    /// List action items.
    ActionItems(ListActionItems),
    /// List people.
    People(ListPeople),
    /// List companies.
    Companies(ListCompanies),
    /// List tags.
    Tags(ListTags),
    /// List folders.
    Folders(ListFolders),
    /// List calendar events.
    Calendar(ListCalendarEvents),
    /// Export a meeting as Markdown, text, JSON, PDF, DOCX or subtitles.
    Export(ExportArgs),
    /// Serve the library to Claude Code and other MCP clients over stdio.
    Mcp,
    /// Show which database is read and whether it can be.
    Status,
}

/// `deino meetings`: list, or `read`.
#[derive(Debug, Args)]
#[command(args_conflicts_with_subcommands = true)]
pub struct MeetingsArgs {
    /// Read meetings instead of listing them.
    #[command(subcommand)]
    pub read: Option<MeetingsCommand>,
    /// What to list.
    #[command(flatten)]
    pub list: ListMeetings,
}

/// `deino meetings read`.
#[derive(Debug, Subcommand)]
pub enum MeetingsCommand {
    /// Read meetings in full: notes, AI notes, action items, attendees, files.
    Read(ReadMeetings),
}

/// `deino transcripts read`.
#[derive(Debug, Subcommand)]
pub enum TranscriptsCommand {
    /// Print a meeting's transcript (every line unless --limit is given).
    Read(GetTranscript),
}

/// Runs `cli` and returns what to print.
pub async fn execute(cli: Cli) -> Result<String, CliError> {
    let path = db_path::resolve(cli.db).ok_or(CliError::NoDataDir)?;
    if matches!(cli.command, Command::Mcp) {
        mcp::serve(path).await?;
        return Ok(String::new());
    }
    if matches!(cli.command, Command::Status) {
        return render(&status(&path).await, cli.json);
    }
    let opened = Opened::read_only(&path).await?;
    dispatch(cli.command, &opened, cli.json).await
}

async fn dispatch(command: Command, opened: &Opened, json: bool) -> Result<String, CliError> {
    let library = &opened.library;
    match command {
        Command::Meetings(MeetingsArgs {
            read: Some(MeetingsCommand::Read(read)),
            ..
        }) => answer(read, library, json).await,
        Command::Meetings(args) => answer(args.list, library, json).await,
        Command::Transcripts(TranscriptsCommand::Read(read)) if read.limit.is_none() => {
            render(&read.read_all(library).await?, json)
        }
        Command::Transcripts(TranscriptsCommand::Read(read)) => answer(read, library, json).await,
        Command::Search(search) => answer(search, library, json).await,
        Command::ActionItems(items) => answer(items, library, json).await,
        Command::People(people) => answer(people, library, json).await,
        Command::Companies(companies) => answer(companies, library, json).await,
        Command::Tags(tags) => answer(tags, library, json).await,
        Command::Folders(folders) => answer(folders, library, json).await,
        Command::Calendar(events) => answer(events, library, json).await,
        Command::Export(export) => export.run(&opened.exports, json).await,
        Command::Mcp | Command::Status => Ok(String::new()),
    }
}

/// Runs a request and renders its output.
async fn answer<R: Request>(
    request: R,
    library: &deino_core::query::Library,
    json: bool,
) -> Result<String, CliError> {
    render(&request.run(library).await?, json)
}

/// `output` as pretty JSON or as Markdown.
pub(crate) fn render<T: Serialize + Markdown>(output: &T, json: bool) -> Result<String, CliError> {
    if json {
        let mut text = serde_json::to_string_pretty(output)?;
        text.push('\n');
        Ok(text)
    } else {
        Ok(output.markdown())
    }
}
