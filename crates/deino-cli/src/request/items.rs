//! `list_action_items` / `deino action-items`.

use deino_core::action_items::{DueFilter, StatusFilter};
use deino_core::query::{ActionItemSummary, Assignee, ItemFilter, Library};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{Request, meeting_id};
use crate::CliError;

/// Which statuses to list.
#[derive(Clone, Copy, Debug, Default, Deserialize, JsonSchema, clap::ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum StatusArg {
    /// Still to do.
    #[default]
    Open,
    /// Done.
    Done,
    /// Both.
    Any,
}

/// Which due days to list, in the user's local days.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, clap::ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum DueArg {
    /// Open items due before today.
    Overdue,
    /// Due today.
    Today,
    /// Due from today to Sunday.
    #[value(name = "this_week", alias = "this-week")]
    ThisWeek,
    /// Without a due day.
    #[value(name = "no_date", alias = "no-date")]
    NoDate,
}

/// Whose items: `me`, `others`, `anyone`, `unassigned`, or a person's
/// name, email or ID.
#[derive(Clone, Debug, Default, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct AssigneeArg(pub String);

impl AssigneeArg {
    fn resolve(&self) -> Assignee {
        match self.0.trim().to_lowercase().as_str() {
            "" | "anyone" | "all" | "everyone" => Assignee::Anyone,
            "me" | "mine" => Assignee::Me,
            "others" => Assignee::Others,
            "unassigned" | "nobody" => Assignee::Unassigned,
            _ => Assignee::Person(self.0.trim().to_owned()),
        }
    }
}

impl std::str::FromStr for AssigneeArg {
    type Err = std::convert::Infallible;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Ok(Self(text.to_owned()))
    }
}

/// List action items.
#[derive(Clone, Debug, Default, Deserialize, JsonSchema, clap::Args)]
#[serde(default, deny_unknown_fields)]
pub struct ListActionItems {
    /// `open` (default), `done` or `any`.
    #[arg(long, value_enum)]
    pub status: Option<StatusArg>,
    /// Whose: `me`, `others`, `anyone` (default), `unassigned`, or a person's name, email or ID.
    #[arg(long)]
    pub assignee: Option<AssigneeArg>,
    /// `overdue`, `today`, `this_week` or `no_date`.
    #[arg(long, value_enum)]
    pub due: Option<DueArg>,
    /// Only this meeting's items.
    #[arg(long)]
    pub meeting_id: Option<String>,
    /// How many items to return (default 50, at most 500).
    #[arg(long, short = 'n')]
    pub limit: Option<u32>,
}

/// Action items, dated ones soonest first.
#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct ActionItemList {
    /// The items that matched.
    pub items: Vec<ActionItemSummary>,
}

impl Request for ListActionItems {
    const NAME: &'static str = "list_action_items";
    const TITLE: &'static str = "List action items";
    const DESCRIPTION: &'static str = "List action items (commitments and to-dos from \
        meetings) with their owner, due day, status, meeting and a link to the moment they \
        were said. Defaults to everyone's open items. Use assignee=me for what the user \
        committed to, status=done or any for finished ones, due=overdue|today|this_week|no_date \
        for deadlines, and meeting_id for one meeting's items.";
    type Output = ActionItemList;

    async fn run(self, library: &Library) -> Result<ActionItemList, CliError> {
        let filter = ItemFilter {
            status: match self.status.unwrap_or_default() {
                StatusArg::Open => StatusFilter::Open,
                StatusArg::Done => StatusFilter::Done,
                StatusArg::Any => StatusFilter::Any,
            },
            assignee: self.assignee.unwrap_or_default().resolve(),
            due: self.due.map(|due| match due {
                DueArg::Overdue => DueFilter::Overdue,
                DueArg::Today => DueFilter::Today,
                DueArg::ThisWeek => DueFilter::ThisWeek,
                DueArg::NoDate => DueFilter::NoDate,
            }),
            meeting: self.meeting_id.as_deref().map(meeting_id).transpose()?,
            limit: self.limit.unwrap_or_default(),
        };
        let items = library.action_items.list(&filter).await?;
        Ok(ActionItemList { items })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assignee_keywords_and_names_resolve() {
        let resolve = |text: &str| AssigneeArg(text.into()).resolve();

        assert_eq!(resolve("ME"), Assignee::Me);
        assert_eq!(resolve(""), Assignee::Anyone);
        assert_eq!(resolve("unassigned"), Assignee::Unassigned);
        assert_eq!(resolve(" Maya "), Assignee::Person("Maya".into()));
    }
}
