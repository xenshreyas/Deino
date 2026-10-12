//! [`FollowUpGenerator`]: the `follow_up_draft` stage's rules, behind the
//! [`FollowUpDrafter`] port the pipeline holds.
//!
//! In order: a dismissed or edited follow-up is kept as it is; the meeting
//! must have an external attendee and at most ten people ([`judge`], no
//! model call otherwise); there must be something to read; the model
//! decides; only a `send` verdict is drafted. Quiet verdicts are stored too,
//! so a re-run that turns quiet replaces an old draft.

use std::sync::Arc;

use async_trait::async_trait;
use deino_domain::{Clock, MeetingId};
use deino_llm::followup::FollowUpDraftAnswer;

use super::{
    FollowUpError, FollowUpInputs, FollowUpModel, FollowUpRepo, FollowUpStatus, Ineligible,
    ParticipantReader, QuietReason, Recipients, StoredFollowUp, judge,
};
use crate::enhance::MaterialSources;
use crate::insights::OwnDomains;

/// The longest subject kept; longer ones are cut.
const MAX_SUBJECT_CHARS: usize = 120;
/// The longest body kept; longer ones are cut.
const MAX_BODY_CHARS: usize = 10_000;

/// Why an existing follow-up was left alone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kept {
    /// The user dismissed it, which is permanent.
    Dismissed,
    /// The user edited the draft.
    Edited,
}

/// What drafting a meeting's follow-up did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FollowUpRun {
    /// A draft is ready.
    Drafted,
    /// The model decided no follow-up was needed.
    Quiet(QuietReason),
    /// The meeting does not qualify; no model was asked.
    Ineligible(Ineligible),
    /// The existing follow-up was left as it is.
    Kept(Kept),
    /// The meeting is gone, or has no transcript or notes.
    NothingToRead,
}

/// Drafts a meeting's follow-up: the `follow_up_draft` stage's port.
#[async_trait]
pub trait FollowUpDrafter: Send + Sync {
    /// Decides on, and maybe drafts, `meeting`'s follow-up email.
    async fn draft(&self, meeting: &MeetingId) -> Result<FollowUpRun, FollowUpError>;
}

/// What [`FollowUpGenerator`] is built from.
pub struct FollowUpDeps {
    /// Stored follow-ups.
    pub store: Arc<dyn FollowUpRepo>,
    /// Who took part.
    pub participants: Arc<dyn ParticipantReader>,
    /// The meeting's transcript and notes, redacted when the setting is on.
    pub sources: MaterialSources,
    /// The user's organisation (Settings → Profile).
    pub domains: Arc<dyn OwnDomains>,
    /// The model.
    pub model: Arc<dyn FollowUpModel>,
    /// When a draft was written.
    pub clock: Arc<dyn Clock>,
}

/// The rules of the `follow_up_draft` stage.
pub struct FollowUpGenerator {
    deps: FollowUpDeps,
}

impl FollowUpGenerator {
    /// A generator over `deps`.
    pub fn new(deps: FollowUpDeps) -> Self {
        Self { deps }
    }

    /// The row to store for `meeting`, or why none is.
    async fn decide(
        &self,
        meeting: &MeetingId,
    ) -> Result<Result<StoredFollowUp, FollowUpRun>, FollowUpError> {
        let people = self.deps.participants.participants(meeting).await?;
        let domains = self.deps.domains.internal_domains().await;
        let to = match judge(&people, &domains) {
            Ok(to) => to,
            Err(why) => return Ok(Err(FollowUpRun::Ineligible(why))),
        };
        let inputs = FollowUpInputs::gather(&self.deps.sources, meeting, &people, &domains).await?;
        let Some(inputs) = inputs else {
            return Ok(Err(FollowUpRun::NothingToRead));
        };
        let decision = self.deps.model.decide(inputs.decide_prompt()).await?;
        let mut row = self.row(
            meeting,
            Recipients {
                to,
                ..Recipients::default()
            },
        );
        if let Some(reason) = QuietReason::of(decision.verdict) {
            return Ok(Ok(quiet(row, reason)));
        }
        let answer = self.deps.model.draft(inputs.draft_prompt()).await?;
        let Some((subject, body)) = cleaned(&answer) else {
            return Ok(Ok(quiet(row, QuietReason::NothingToFollowUp)));
        };
        row.subject = subject;
        row.body = body;
        Ok(Ok(row))
    }

    /// A ready row for `meeting`, written now, without text yet.
    fn row(&self, meeting: &MeetingId, recipients: Recipients) -> StoredFollowUp {
        StoredFollowUp {
            meeting_id: meeting.clone(),
            status: FollowUpStatus::Ready,
            quiet_reason: None,
            subject: String::new(),
            body: String::new(),
            recipients,
            generated_at: self.deps.clock.now(),
            edited_at: None,
            dismissed_at: None,
        }
    }
}

#[async_trait]
impl FollowUpDrafter for FollowUpGenerator {
    async fn draft(&self, meeting: &MeetingId) -> Result<FollowUpRun, FollowUpError> {
        if let Some(kept) = self
            .deps
            .store
            .find(meeting)
            .await?
            .and_then(|row| kept(&row))
        {
            return Ok(FollowUpRun::Kept(kept));
        }
        let row = match self.decide(meeting).await? {
            Ok(row) => row,
            Err(run) => return Ok(run),
        };
        if !self.deps.store.save(&row).await? {
            // Dismissed or edited while the model was writing.
            return Ok(FollowUpRun::Kept(Kept::Edited));
        }
        log::info!(target: "followups", "follow-up for meeting {meeting}: {:?}", row.status);
        Ok(match row.quiet_reason {
            Some(reason) => FollowUpRun::Quiet(reason),
            None => FollowUpRun::Drafted,
        })
    }
}

/// Why a stored follow-up must not be replaced, if it must not.
fn kept(row: &StoredFollowUp) -> Option<Kept> {
    if row.status == FollowUpStatus::Dismissed {
        return Some(Kept::Dismissed);
    }
    row.edited_at.map(|_| Kept::Edited)
}

/// `row` as a quiet decision: no text, the reason kept.
fn quiet(row: StoredFollowUp, reason: QuietReason) -> StoredFollowUp {
    StoredFollowUp {
        status: FollowUpStatus::Quiet,
        quiet_reason: Some(reason),
        subject: String::new(),
        body: String::new(),
        ..row
    }
}

/// The answer's subject and body, trimmed and cut to size; `None` when the
/// body is empty.
fn cleaned(answer: &FollowUpDraftAnswer) -> Option<(String, String)> {
    let body: String = answer.body.trim().chars().take(MAX_BODY_CHARS).collect();
    if body.is_empty() {
        return None;
    }
    let subject = answer.subject.trim();
    let subject = subject.strip_prefix("Re: ").unwrap_or(subject);
    let subject: String = subject
        .lines()
        .next()
        .unwrap_or_default()
        .chars()
        .take(MAX_SUBJECT_CHARS)
        .collect();
    Some((subject, body))
}
