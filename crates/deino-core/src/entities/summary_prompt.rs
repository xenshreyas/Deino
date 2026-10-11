//! The `entity.summary` prompt: pure, so tests check it without a model.
//!
//! A briefing reads the subject's details and the user's notes about them,
//! their latest [`SUMMARY_MEETINGS`] meetings (each meeting's summary, else
//! the user's notes, cut to [`MEETING_CHARS`]) and their open action items.
//! Meetings with neither are left out; with no meeting text and no notes
//! there is nothing to brief from, and no prompt.

use deino_llm::Prompt;
use deino_llm::prompts::{EntityMeetingText, EntitySummarySystem, EntitySummaryUser, PromptText};

use super::{EntityActionItem, EntityDigest};

/// Meetings a briefing reads.
pub const SUMMARY_MEETINGS: u32 = 8;
/// Characters of each meeting's summary or notes in the prompt.
pub const MEETING_CHARS: usize = 1_500;
/// Characters of the user's notes about the subject in the prompt.
const NOTES_CHARS: usize = 2_000;
/// Open action items listed, at most.
const MAX_ITEMS: usize = 10;
/// Room for the answer: under 150 words.
const ANSWER_TOKENS: u32 = 400;

/// Whom a briefing is about.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubjectKind {
    /// A person.
    Person,
    /// A company.
    Company,
}

impl SubjectKind {
    fn word(self) -> &'static str {
        match self {
            Self::Person => "person",
            Self::Company => "company",
        }
    }
}

/// What a briefing knows about its subject before reading meetings.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Subject {
    /// A person or a company.
    pub kind: SubjectKind,
    /// The name.
    pub name: String,
    /// `Label: value` lines: title, company, domain, people.
    pub details: Vec<String>,
    /// The user's notes about them.
    pub notes: String,
}

/// The `entity.summary` prompt for `subject`, or `None` when nothing has
/// been written about them yet.
pub fn summary_prompt(
    subject: &Subject,
    digests: &[EntityDigest],
    open_items: &[EntityActionItem],
) -> Option<Prompt> {
    let texts: Vec<(String, String, &EntityDigest)> = digests
        .iter()
        .filter_map(|digest| {
            Some((
                content(digest)?,
                digest.at.format("%Y-%m-%d").to_string(),
                digest,
            ))
        })
        .collect();
    let notes = cut(subject.notes.trim(), NOTES_CHARS);
    if texts.is_empty() && notes.is_empty() {
        return None;
    }
    let meetings: Vec<EntityMeetingText<'_>> = texts
        .iter()
        .map(|(content, date, digest)| EntityMeetingText {
            title: &digest.title,
            date,
            content,
        })
        .collect();
    let items: Vec<String> = open_items.iter().take(MAX_ITEMS).map(item_line).collect();
    let user = EntitySummaryUser {
        name: &subject.name,
        details: &subject.details,
        notes: (!notes.is_empty()).then_some(notes.as_str()),
        meetings: &meetings,
        open_items: &items,
    };
    let system = EntitySummarySystem {
        kind: subject.kind.word(),
    };
    Some(
        Prompt::new(user.text())
            .system(system.text())
            .max_tokens(ANSWER_TOKENS),
    )
}

/// The meeting's summary, else the user's notes, cut to the budget.
fn content(digest: &EntityDigest) -> Option<String> {
    let text = [&digest.summary, &digest.notes]
        .into_iter()
        .flatten()
        .map(|text| text.trim())
        .find(|text| !text.is_empty())?;
    Some(cut(text, MEETING_CHARS))
}

/// `text` cut to `limit` characters, marked with `…` when cut.
fn cut(text: &str, limit: usize) -> String {
    let mut chars = text.chars();
    let mut kept: String = chars.by_ref().take(limit).collect();
    if chars.next().is_some() {
        kept.push('…');
    }
    kept
}

/// One open item: its title, with the due date when it has one.
fn item_line(item: &EntityActionItem) -> String {
    match item.due_at {
        Some(due) => format!("{} (due {})", item.title, due.format("%Y-%m-%d")),
        None => item.title.clone(),
    }
}

#[cfg(test)]
mod tests {
    use deino_domain::{ActionItemId, FakeClock, MeetingId};
    use deino_llm::Role;
    use deino_testkit::at;

    use super::*;

    fn digest(title: &str, summary: Option<&str>, notes: Option<&str>) -> EntityDigest {
        EntityDigest {
            meeting_id: MeetingId::new(format!("meeting-{title}")),
            title: title.into(),
            at: at("2026-10-07T10:00:00Z"),
            summary: summary.map(str::to_owned),
            notes: notes.map(str::to_owned),
        }
    }

    fn maya() -> Subject {
        Subject {
            kind: SubjectKind::Person,
            name: "Maya Chen".into(),
            details: vec!["Company: Deino".into()],
            notes: String::new(),
        }
    }

    fn user_text(prompt: &Prompt) -> String {
        let message = prompt.messages.first().unwrap();
        assert_eq!(message.role, Role::User);
        message.text()
    }

    #[test]
    fn meetings_use_their_summary_else_the_users_notes_and_skip_empty_ones() {
        let digests = [
            digest("1:1", Some("Split the webhook queue."), Some("ignored")),
            digest("Standup", None, Some("  Maya blocked on review.  ")),
            digest("Empty", Some("  "), None),
        ];
        let item = EntityActionItem {
            id: ActionItemId::generate(&FakeClock::new(at("2026-10-07T10:00:00Z"))),
            title: "Draft the queue design".into(),
            due_at: Some(at("2026-10-14T17:00:00Z")),
            assignee_person_id: None,
            meeting_id: None,
        };

        let prompt = summary_prompt(&maya(), &digests, &[item]).unwrap();

        let user = user_text(&prompt);
        assert!(
            user.contains("## 1:1 (2026-10-07)\nSplit the webhook queue."),
            "{user}"
        );
        assert!(
            user.contains("## Standup (2026-10-07)\nMaya blocked on review."),
            "{user}"
        );
        assert!(!user.contains("Empty"), "{user}");
        assert!(
            user.contains("- Draft the queue design (due 2026-10-14)"),
            "{user}"
        );
        assert!(prompt.system.unwrap().contains("one person the user meets"));
        assert_eq!(prompt.max_tokens, Some(ANSWER_TOKENS));
    }

    #[test]
    fn nothing_written_about_them_means_no_prompt() {
        assert!(summary_prompt(&maya(), &[digest("Silent", None, None)], &[]).is_none());
        let mut noted = maya();
        noted.notes = "Met at a conference.".into();
        assert!(summary_prompt(&noted, &[], &[]).is_some());
    }

    #[test]
    fn long_meetings_are_cut_with_a_mark() {
        let long = "word ".repeat(MEETING_CHARS);

        let prompt = summary_prompt(&maya(), &[digest("Long", Some(&long), None)], &[]).unwrap();

        let user = user_text(&prompt);
        assert!(user.contains('…'));
        assert!(user.chars().count() < MEETING_CHARS + 500);
    }
}
