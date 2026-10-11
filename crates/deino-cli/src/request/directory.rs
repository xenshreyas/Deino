//! The directory: `list_people`, `list_companies`, `list_tags`,
//! `list_folders` and `list_calendar_events` (`deino people`, `companies`,
//! `tags`, `folders`, `calendar`).

use chrono::NaiveDate;
use deino_core::query::{
    CompanyView, EventView, EventWindow, FolderView, Library, PersonView, TagView,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::Request;
use crate::CliError;

/// List people.
#[derive(Clone, Debug, Default, Deserialize, JsonSchema, clap::Args)]
#[serde(default, deny_unknown_fields)]
pub struct ListPeople {
    /// Only people whose name or email contains this text.
    pub query: Option<String>,
    /// How many people to return (default 100, at most 1000).
    #[arg(long, short = 'n')]
    pub limit: Option<u32>,
}

/// People, by name.
#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct PeopleList {
    /// The people that matched.
    pub people: Vec<PersonView>,
}

impl Request for ListPeople {
    const NAME: &'static str = "list_people";
    const TITLE: &'static str = "List people";
    const DESCRIPTION: &'static str = "List the people the user has met or added, with \
        email, job title and company; query filters by part of a name or email. Use it to \
        find the exact name or email to pass as person to list_meetings or as assignee to \
        list_action_items.";
    type Output = PeopleList;

    async fn run(self, library: &Library) -> Result<PeopleList, CliError> {
        let people = library
            .directory
            .people(self.query.as_deref(), self.limit.unwrap_or_default())
            .await?;
        Ok(PeopleList { people })
    }
}

/// List companies.
#[derive(Clone, Debug, Default, Deserialize, JsonSchema, clap::Args)]
#[serde(default, deny_unknown_fields)]
pub struct ListCompanies {}

/// Companies, by name.
#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct CompanyList {
    /// Every company.
    pub companies: Vec<CompanyView>,
}

impl Request for ListCompanies {
    const NAME: &'static str = "list_companies";
    const TITLE: &'static str = "List companies";
    const DESCRIPTION: &'static str = "List the companies of the people the user meets, with \
        their email domain and how many known people work there. Pass a name or domain as \
        company to list_meetings to find meetings with that company.";
    type Output = CompanyList;

    async fn run(self, library: &Library) -> Result<CompanyList, CliError> {
        let companies = library.directory.companies().await?;
        Ok(CompanyList { companies })
    }
}

/// List tags.
#[derive(Clone, Debug, Default, Deserialize, JsonSchema, clap::Args)]
#[serde(default, deny_unknown_fields)]
pub struct ListTags {}

/// Tags, by name.
#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct TagList {
    /// Every tag.
    pub tags: Vec<TagView>,
}

impl Request for ListTags {
    const NAME: &'static str = "list_tags";
    const TITLE: &'static str = "List tags";
    const DESCRIPTION: &'static str = "List the user's meeting tags with their description \
        and how many meetings carry each. Pass a tag's name as tag to list_meetings.";
    type Output = TagList;

    async fn run(self, library: &Library) -> Result<TagList, CliError> {
        let tags = library.directory.tags().await?;
        Ok(TagList { tags })
    }
}

/// List folders.
#[derive(Clone, Debug, Default, Deserialize, JsonSchema, clap::Args)]
#[serde(default, deny_unknown_fields)]
pub struct ListFolders {}

/// Folders, in tree order.
#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct FolderList {
    /// Every folder.
    pub folders: Vec<FolderView>,
}

impl Request for ListFolders {
    const NAME: &'static str = "list_folders";
    const TITLE: &'static str = "List folders";
    const DESCRIPTION: &'static str = "List the user's meeting folders as a tree (each with \
        its path, such as \"Customers / Acme\", and how many meetings it holds). Pass a \
        folder's name as folder to list_meetings; subfolders are included.";
    type Output = FolderList;

    async fn run(self, library: &Library) -> Result<FolderList, CliError> {
        let folders = library.directory.folders().await?;
        Ok(FolderList { folders })
    }
}

/// List calendar events.
#[derive(Clone, Debug, Default, Deserialize, JsonSchema, clap::Args)]
#[serde(default, deny_unknown_fields)]
pub struct ListCalendarEvents {
    /// The first local day (YYYY-MM-DD); default a week ago.
    #[arg(long)]
    pub from: Option<NaiveDate>,
    /// The last local day (YYYY-MM-DD), inclusive; default a week from today.
    #[arg(long)]
    pub to: Option<NaiveDate>,
}

/// Calendar events, by start time.
#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct EventList {
    /// The events starting in the window.
    pub events: Vec<EventView>,
}

impl Request for ListCalendarEvents {
    const NAME: &'static str = "list_calendar_events";
    const TITLE: &'static str = "List calendar events";
    const DESCRIPTION: &'static str = "List the user's synced calendar events (past and \
        upcoming) starting between from and to (YYYY-MM-DD, local days, at most 92 days; \
        default a week either side of today), with times, call link, organiser, invitees and \
        the user's RSVP. Use it for the schedule; recorded meetings come from list_meetings.";
    type Output = EventList;

    async fn run(self, library: &Library) -> Result<EventList, CliError> {
        let window = EventWindow {
            from: self.from,
            to: self.to,
        };
        let events = library.directory.events(window).await?;
        Ok(EventList { events })
    }
}
