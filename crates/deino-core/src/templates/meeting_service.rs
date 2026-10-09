//! [`MeetingTemplateService`]: a meeting's template, why it applies, how to
//! change it, and what a template's prompt looks like for that meeting.

use std::sync::Arc;

use deino_domain::{Clock, MeetingId, TemplateId};

use super::selection::Selection;
use super::{
    ChoiceReason, MeetingFacts, MeetingFactsReader, TemplateChoice, TemplateClassifier,
    TemplateContent, TemplateError, TemplatePreview, TemplateRepo, preview,
};
use crate::meetings::MeetingLifecycleRepo;
use crate::profile::{InternalDomains, render_profile_context};
use crate::settings::SettingsService;

/// What the service is built from.
pub struct MeetingTemplatePorts {
    /// The templates.
    pub templates: Arc<dyn TemplateRepo>,
    /// Titles and attendees of meetings.
    pub facts: Arc<dyn MeetingFactsReader>,
    /// Where a meeting's chosen template is saved.
    pub meetings: Arc<dyn MeetingLifecycleRepo>,
    /// The default template and the profile (internal domains, context).
    pub settings: Arc<SettingsService>,
    /// The LLM fallback.
    pub classifier: Arc<dyn TemplateClassifier>,
    /// Time for `updated_at`.
    pub clock: Arc<dyn Clock>,
}

/// Chooses, changes and previews meetings' templates.
pub struct MeetingTemplateService {
    ports: MeetingTemplatePorts,
}

impl MeetingTemplateService {
    /// Creates the service over its ports.
    pub fn new(ports: MeetingTemplatePorts) -> Self {
        Self { ports }
    }

    /// The template `meeting` gets and why, without calling an LLM: the
    /// user's choice, an auto rule, the default, or General. What the UI shows.
    pub async fn choice(&self, meeting: &MeetingId) -> Result<TemplateChoice, TemplateError> {
        self.select(meeting, false).await
    }

    /// Like [`choice`](Self::choice), but asks the LLM classifier when no
    /// rule matched and the default template is "Auto". For the post-meeting
    /// pipeline (NOT-02), which is about to call an LLM anyway.
    pub async fn auto_choice(&self, meeting: &MeetingId) -> Result<TemplateChoice, TemplateError> {
        self.select(meeting, true).await
    }

    /// Chooses `template` for `meeting`, or clears the choice (`None`) so
    /// selection is automatic again. Returns the meeting's new choice.
    pub async fn set(
        &self,
        meeting: &MeetingId,
        template: Option<&TemplateId>,
    ) -> Result<TemplateChoice, TemplateError> {
        if let Some(id) = template
            && self.ports.templates.get(id).await?.is_none()
        {
            return Err(TemplateError::NotFound(id.clone()));
        }
        let now = self.ports.clock.now();
        if !self
            .ports
            .meetings
            .set_template(meeting, template, now)
            .await?
        {
            return Err(TemplateError::MeetingNotFound(meeting.clone()));
        }
        log::debug!(target: "templates", "meeting {meeting} template set");
        self.choice(meeting).await
    }

    /// `content` (saved or not) rendered for `meeting`, without an LLM call.
    pub async fn preview(
        &self,
        content: TemplateContent,
        meeting: &MeetingId,
    ) -> Result<TemplatePreview, TemplateError> {
        let content = content.validated()?;
        let facts = self.facts(meeting).await?;
        let profile = self.ports.settings.get().await.profile;
        let audience = InternalDomains::of(&profile).classify(&facts.attendee_emails);
        Ok(preview(
            &content,
            &facts,
            audience,
            render_profile_context(&profile),
        ))
    }

    async fn select(
        &self,
        meeting: &MeetingId,
        ask_llm: bool,
    ) -> Result<TemplateChoice, TemplateError> {
        let facts = self.facts(meeting).await?;
        let templates = self.ports.templates.list().await?;
        let settings = self.ports.settings.get().await;
        let domains = InternalDomains::of(&settings.profile);
        let selection = Selection {
            templates: &templates,
            meeting: &facts,
            domains: &domains,
        };
        let default = settings.templates.default_template.as_ref();
        if let Some(choice) = selection
            .chosen_or_rule()
            .or_else(|| selection.default(default))
        {
            return Ok(choice);
        }
        if ask_llm && let Some(choice) = self.classified(&selection).await {
            return Ok(choice);
        }
        selection.fallback().ok_or(TemplateError::NoTemplates)
    }

    /// The classifier's pick, if it made a valid one. A failure only logs:
    /// selection never fails because the LLM did.
    async fn classified(&self, selection: &Selection<'_>) -> Option<TemplateChoice> {
        let result = self
            .ports
            .classifier
            .classify(selection.meeting, selection.templates)
            .await;
        let classification = match result {
            Ok(classification) => classification?,
            Err(error) => {
                log::warn!(target: "templates", "template classifier failed: {error}");
                return None;
            }
        };
        let template = selection.find(&classification.template_id)?.clone();
        Some(TemplateChoice {
            template,
            reason: ChoiceReason::Classified {
                explanation: classification.explanation,
            },
        })
    }

    async fn facts(&self, meeting: &MeetingId) -> Result<MeetingFacts, TemplateError> {
        self.ports
            .facts
            .facts(meeting)
            .await?
            .ok_or_else(|| TemplateError::MeetingNotFound(meeting.clone()))
    }
}
