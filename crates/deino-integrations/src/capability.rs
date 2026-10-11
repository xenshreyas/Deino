//! The capability model: what a provider can do, as one small trait each.
//!
//! An [`Integration`](crate::Integration) offers a capability by returning
//! `Some(self)` from the matching accessor (`messages()`, `pages()`, ...).
//! Callers never ask "is this Slack?": an automation step holds an
//! [`Action`] whose payload type is fixed by its capability, and the
//! delivery engine dispatches it to whichever provider the user picked.
//! Adding a capability is one trait here, one [`Action`] variant and one
//! `Capability` value in `deino-domain`.

use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use deino_domain::{Capability, Email};
use serde::{Deserialize, Serialize};

use crate::{ExternalRef, ProviderApi, ProviderError};

/// A place a provider can write to: a Slack channel, a Notion database, a
/// Linear team. Its ID is opaque and only means something to its provider.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Target {
    /// The provider's ID for it.
    pub id: String,
    /// What the user sees: `#design`, `Meeting notes`, `Platform team`.
    pub name: String,
}

/// A message, with optional replies posted as a thread under it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct MessageDraft {
    /// Where to post ([`Target::id`]).
    pub target: String,
    /// The parent message, as Markdown.
    pub text: String,
    /// Replies in its thread, in order, as Markdown.
    pub replies: Vec<String>,
}

/// A page or document under a parent page, database or folder.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PageDraft {
    /// Where to create it ([`Target::id`]).
    pub parent: String,
    /// The page title.
    pub title: String,
    /// The body, as Markdown.
    pub body: String,
}

/// A task in a project, team or list.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TaskDraft {
    /// Where to create it ([`Target::id`]).
    pub project: String,
    /// The task title.
    pub title: String,
    /// The description, as Markdown.
    pub description: String,
    /// The due date, if any.
    pub due_on: Option<NaiveDate>,
    /// Who it is for; the provider matches the account by email.
    pub assignee: Option<Email>,
}

/// A meeting or note logged on the CRM records of its attendees.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ActivityDraft {
    /// The activity title.
    pub title: String,
    /// The body, as Markdown.
    pub body: String,
    /// When the meeting took place.
    pub occurred_at: DateTime<Utc>,
    /// The attendees whose records it is logged on.
    pub attendees: Vec<Email>,
}

/// A request for context: recent items that match `text`. Results are used
/// once and never stored.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ContextQuery {
    /// What to look for: words, names, email addresses.
    pub text: String,
    /// The most items to return.
    pub limit: u16,
}

/// One item of context: an email thread, an issue, a page.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ContextItem {
    /// Its title or subject.
    pub title: String,
    /// A short excerpt.
    pub snippet: String,
    /// Where to open it.
    pub url: Option<String>,
}

/// One write to a provider, typed by its capability.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "capability", content = "draft", rename_all = "snake_case")]
pub enum Action {
    /// Post a message.
    PostMessage(MessageDraft),
    /// Create a page.
    CreatePage(PageDraft),
    /// Create a task.
    CreateTask(TaskDraft),
    /// Log an activity.
    LogActivity(ActivityDraft),
}

impl Action {
    /// The capability this action needs.
    pub const fn capability(&self) -> Capability {
        match self {
            Self::PostMessage(_) => Capability::PostMessage,
            Self::CreatePage(_) => Capability::CreatePage,
            Self::CreateTask(_) => Capability::CreateTask,
            Self::LogActivity(_) => Capability::LogActivity,
        }
    }
}

/// Posts messages (Slack, email-to-self).
#[async_trait]
pub trait PostMessage: Send + Sync {
    /// The channels and conversations the account can post to.
    async fn message_targets(&self, api: &ProviderApi) -> Result<Vec<Target>, ProviderError>;

    /// Posts `draft` and its replies; returns the parent message.
    async fn post_message(
        &self,
        api: &ProviderApi,
        draft: &MessageDraft,
    ) -> Result<ExternalRef, ProviderError>;
}

/// Creates pages (Notion, Google Docs).
#[async_trait]
pub trait CreatePage: Send + Sync {
    /// The pages, databases or folders new pages can go under.
    async fn page_targets(&self, api: &ProviderApi) -> Result<Vec<Target>, ProviderError>;

    /// Creates the page.
    async fn create_page(
        &self,
        api: &ProviderApi,
        draft: &PageDraft,
    ) -> Result<ExternalRef, ProviderError>;
}

/// Creates tasks (Linear, Todoist, Asana, ...).
#[async_trait]
pub trait CreateTask: Send + Sync {
    /// The projects, teams or lists tasks can go into.
    async fn task_targets(&self, api: &ProviderApi) -> Result<Vec<Target>, ProviderError>;

    /// Creates the task.
    async fn create_task(
        &self,
        api: &ProviderApi,
        draft: &TaskDraft,
    ) -> Result<ExternalRef, ProviderError>;
}

/// Logs activities on CRM records (HubSpot, Attio, ...).
#[async_trait]
pub trait LogActivity: Send + Sync {
    /// Logs the activity on the attendees' records.
    async fn log_activity(
        &self,
        api: &ProviderApi,
        draft: &ActivityDraft,
    ) -> Result<ExternalRef, ProviderError>;
}

/// Reads context on demand (Gmail threads, Linear issues).
#[async_trait]
pub trait ReadContext: Send + Sync {
    /// Items matching `query`, most relevant first.
    async fn read_context(
        &self,
        api: &ProviderApi,
        query: &ContextQuery,
    ) -> Result<Vec<ContextItem>, ProviderError>;
}
