//! From the model's answer to stored items: [`resolve_items`] validates and
//! resolves each extracted item (title, assignee, due day, evidence), and
//! [`without_repeats`] drops those the meeting already has in a form the
//! user touched, so regenerating never duplicates a kept item.

use deino_domain::{
    ActionItemId, ActionItemOrigin, ActionItemPriority, ActionItemStatus, Clock, LocalZone,
    MeetingId, NOON,
};
use deino_llm::extraction::{ExtractedActionItem, ExtractedPriority};

use super::assignee::normalize;
use super::context::MeetingContext;
use super::due::resolve_due;
use crate::action_items::{ActionItem, MAX_DESCRIPTION_CHARS, MAX_TITLE_CHARS};

/// What resolution needs besides the answer.
#[derive(Clone, Copy)]
pub struct Resolution<'a> {
    /// The meeting the items belong to.
    pub meeting: &'a MeetingId,
    /// Its day, transcript and people.
    pub context: &'a MeetingContext,
    /// The user's time zone, for storing due days.
    pub zone: &'a dyn LocalZone,
    /// Creation time and IDs.
    pub clock: &'a dyn Clock,
}

/// The answer's items as new, untouched AI items; items without a usable
/// title are dropped.
pub fn resolve_items(extracted: &[ExtractedActionItem], with: Resolution<'_>) -> Vec<ActionItem> {
    let now = with.clock.now();
    extracted
        .iter()
        .filter_map(|item| {
            let title = clean_title(&item.title)?;
            let due = resolve_due(
                item.due_phrase.as_deref(),
                item.due.as_deref(),
                with.context.day,
            );
            Some(ActionItem {
                id: ActionItemId::generate(with.clock),
                meeting_id: Some(with.meeting.clone()),
                title,
                description: clip(item.description.trim(), MAX_DESCRIPTION_CHARS),
                assignee_person_id: item
                    .assignee
                    .as_deref()
                    .and_then(|name| with.context.roster.resolve(name)),
                due_at: due.map(|day| with.zone.at(day, NOON)),
                status: ActionItemStatus::Open,
                priority: item.priority.map(priority),
                source_segment_id: with
                    .context
                    .transcript
                    .locate(item.evidence_segment_id.as_deref(), &item.evidence_quote),
                completed_at: None,
                created_by: ActionItemOrigin::Ai,
                created_at: now,
                updated_at: now,
            })
        })
        .collect()
}

/// `fresh` without the items that repeat one `existing` keeps (any the
/// user touched: the same evidence line, or the same title ignoring case and
/// punctuation) or an earlier fresh item (the same title; one line may hold
/// two commitments).
pub fn without_repeats(fresh: Vec<ActionItem>, existing: &[ActionItem]) -> Vec<ActionItem> {
    let touched: Vec<&ActionItem> = existing
        .iter()
        .filter(|item| !item.is_untouched_ai())
        .collect();
    let mut kept: Vec<ActionItem> = Vec::with_capacity(fresh.len());
    for item in fresh {
        let known = touched
            .iter()
            .any(|other| same_line(&item, other) || same_title(&item, other))
            || kept.iter().any(|other| same_title(&item, other));
        if !known {
            kept.push(item);
        }
    }
    kept
}

fn same_line(item: &ActionItem, other: &ActionItem) -> bool {
    item.source_segment_id.is_some() && item.source_segment_id == other.source_segment_id
}

fn same_title(item: &ActionItem, other: &ActionItem) -> bool {
    normalize(&item.title) == normalize(&other.title)
}

/// A one-line title without a trailing period, clipped; `None` when blank.
fn clean_title(title: &str) -> Option<String> {
    let line = title
        .lines()
        .next()
        .unwrap_or_default()
        .trim()
        .trim_end_matches('.')
        .trim();
    (!line.is_empty()).then(|| clip(line, MAX_TITLE_CHARS))
}

fn clip(text: &str, max_chars: usize) -> String {
    text.chars().take(max_chars).collect()
}

fn priority(said: ExtractedPriority) -> ActionItemPriority {
    match said {
        ExtractedPriority::Low => ActionItemPriority::Low,
        ExtractedPriority::Medium => ActionItemPriority::Medium,
        ExtractedPriority::High => ActionItemPriority::High,
    }
}
