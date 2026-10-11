//! Action items as Markdown task lines.

use deino_core::query::ActionItemSummary;
use deino_domain::ActionItemStatus;

use super::{day, line};
use crate::request::{ActionItemList, Markdown};

impl Markdown for ActionItemList {
    fn markdown(&self) -> String {
        let mut out = String::new();
        if self.items.is_empty() {
            line(&mut out, "No action items match.");
            return out;
        }
        line(&mut out, format!("# Action items ({})\n", self.items.len()));
        for item in &self.items {
            line(&mut out, item_line(item, true));
        }
        out
    }
}

/// `- [ ] Title — Owner · due Fri 9 Oct 2026 · Meeting · link`, with the
/// meeting's title when `with_meeting` (a list across meetings).
pub(crate) fn item_line(item: &ActionItemSummary, with_meeting: bool) -> String {
    let check = if item.status == ActionItemStatus::Done {
        "x"
    } else {
        " "
    };
    let mut parts = Vec::new();
    match (&item.assignee, item.assignee_is_me) {
        (Some(name), true) => parts.push(format!("{name} (me)")),
        (Some(name), false) => parts.push(name.clone()),
        (None, _) => parts.push("unassigned".to_owned()),
    }
    if let Some(due) = item.due {
        parts.push(format!("due {}", day(due)));
    }
    if let (true, Some(title)) = (with_meeting, &item.meeting_title) {
        parts.push(title.clone());
    }
    if let Some(link) = &item.link {
        parts.push(link.clone());
    }
    let mut text = format!("- [{check}] {} — {}", item.title, parts.join(" · "));
    if !item.description.trim().is_empty() {
        text.push_str("\n  ");
        text.push_str(item.description.trim());
    }
    text
}
