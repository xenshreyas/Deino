//! Which template a meeting gets, and why: the pure steps of selection.
//!
//! In order: the template the user chose for the meeting; the best auto rule
//! ([`RuleEngine`]); the default template from Settings; the LLM classifier
//! (only when the default is "Auto", and only in the post-meeting pipeline);
//! the General template. The classifier step is the service's, since it is
//! the only one that is not pure.

use deino_domain::{Audience, TemplateId};
use serde::Serialize;

use super::{GENERAL_TEMPLATE_ID, MeetingFacts, MeetingSignals, RuleEngine, Template};
use crate::profile::InternalDomains;

/// Why a meeting has its template.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ChoiceReason {
    /// The user chose it for this meeting.
    Chosen,
    /// An auto rule matched; `rule` is its label or description.
    Rule {
        /// "external discovery call".
        rule: String,
    },
    /// The LLM classifier suggested it.
    Classified {
        /// The classifier's short reason.
        explanation: String,
    },
    /// The default template in Settings.
    Default,
    /// Nothing else applied, so the General template.
    Fallback,
}

/// A meeting's template and the reason it applies.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TemplateChoice {
    /// The template.
    pub template: Template,
    /// Why it applies.
    pub reason: ChoiceReason,
}

/// Everything the pure steps read.
pub(super) struct Selection<'a> {
    /// Every template, in list order.
    pub templates: &'a [Template],
    /// The meeting.
    pub meeting: &'a MeetingFacts,
    /// The user's organisation, for internal vs external.
    pub domains: &'a InternalDomains,
}

impl Selection<'_> {
    /// Internal or external, when it can be told.
    pub(super) fn audience(&self) -> Option<Audience> {
        self.domains.classify(&self.meeting.attendee_emails)
    }

    /// The user's choice, else the best auto rule.
    pub(super) fn chosen_or_rule(&self) -> Option<TemplateChoice> {
        if let Some(chosen) = self
            .meeting
            .template_id
            .as_ref()
            .and_then(|id| self.find(id))
        {
            return Some(choice(chosen, ChoiceReason::Chosen));
        }
        let signals = MeetingSignals {
            title: &self.meeting.title,
            attendee_emails: &self.meeting.attendee_emails,
            attendee_count: self.meeting.attendee_count,
            audience: self.audience(),
        };
        let matched = RuleEngine::new(self.templates).best_match(&signals)?;
        let template = self.find(&matched.template_id)?;
        Some(choice(template, ChoiceReason::Rule { rule: matched.rule }))
    }

    /// The default template from Settings, if it still exists.
    pub(super) fn default(&self, default: Option<&TemplateId>) -> Option<TemplateChoice> {
        let template = self.find(default?)?;
        Some(choice(template, ChoiceReason::Default))
    }

    /// The General template, else the first one there is.
    pub(super) fn fallback(&self) -> Option<TemplateChoice> {
        let general = TemplateId::new(GENERAL_TEMPLATE_ID);
        let template = self.find(&general).or_else(|| self.templates.first())?;
        Some(choice(template, ChoiceReason::Fallback))
    }

    /// The template with `id`.
    pub(super) fn find(&self, id: &TemplateId) -> Option<&Template> {
        self.templates.iter().find(|template| template.id == *id)
    }
}

fn choice(template: &Template, reason: ChoiceReason) -> TemplateChoice {
    TemplateChoice {
        template: template.clone(),
        reason,
    }
}
