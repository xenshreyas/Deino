//! Auto-tagging: the `auto_tag` pipeline stage's work (order 70, ORG-06).
//!
//! After a meeting is processed, the `tagging` route picks which of the
//! user's existing tags fit it (see [`classifier`](super::classifier)). The
//! auto-tagger stays out of the way: it runs only while Settings allows it,
//! never on a meeting the user tagged by hand, and only offers tags whose
//! auto-apply switch is on. Applied tags are recorded as `ai` and announced
//! like any other ([`MeetingTagged`](super::MeetingTagged)).

use std::sync::Arc;

use async_trait::async_trait;
use deino_domain::{MeetingId, TagId, TagSource};
use deino_llm::{Gateway, Route};

use super::classifier::{
    Candidate, MAX_EXAMPLES, MeetingContent, TaggingAnswer, chosen_tags, tagging_prompt,
};
use super::{
    CONTENT_CHARS, MeetingDigest, MeetingDigestReader, MeetingTagRepo, TagError, TagRepo,
    TagService,
};
use crate::settings::SettingsService;

/// How auto-tagging a meeting went.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AutoTagged {
    /// The model chose these tags (possibly none); new ones were applied.
    Applied(Vec<TagId>),
    /// Auto-tagging did not run, for this reason.
    Skipped(AutoTagSkip),
}

/// Why auto-tagging did not run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AutoTagSkip {
    /// Settings turn auto-tagging off.
    Disabled,
    /// The user tagged the meeting by hand.
    TaggedByUser,
    /// No tag may be applied automatically.
    NoTags,
    /// The meeting has no summary or transcript to read.
    NoContent,
    /// The meeting is gone or in the trash.
    MeetingGone,
    /// No LLM could be wired at startup.
    NoModel,
}

impl AutoTagSkip {
    /// A user-safe sentence for the meeting page.
    pub fn reason(self) -> &'static str {
        match self {
            Self::Disabled => "Auto-tagging is off in Settings.",
            Self::TaggedByUser => "You tagged this meeting yourself.",
            Self::NoTags => "There are no tags for Deino to apply.",
            Self::NoContent => "The meeting has no notes or transcript to tag from.",
            Self::MeetingGone => "The meeting was deleted.",
            Self::NoModel => "No AI model is available to choose tags.",
        }
    }
}

/// Whether auto-tagging is on (`Settings::organize.auto_tag`).
#[async_trait]
pub trait AutoTagSwitch: Send + Sync {
    /// True when the auto-tagger may run.
    async fn auto_tag_enabled(&self) -> bool;
}

#[async_trait]
impl AutoTagSwitch for SettingsService {
    async fn auto_tag_enabled(&self) -> bool {
        self.get().await.organize.auto_tag
    }
}

/// Applies existing tags to a processed meeting.
#[async_trait]
pub trait AutoTagger: Send + Sync {
    /// Tags `meeting` (see the module docs). Dropping the future cancels
    /// the model call.
    async fn auto_tag(&self, meeting: &MeetingId) -> Result<AutoTagged, TagError>;
}

/// The [`AutoTagger`] for an app without an LLM gateway: it always skips
/// with [`AutoTagSkip::NoModel`].
pub struct NoModelAutoTagger;

#[async_trait]
impl AutoTagger for NoModelAutoTagger {
    async fn auto_tag(&self, _meeting: &MeetingId) -> Result<AutoTagged, TagError> {
        Ok(AutoTagged::Skipped(AutoTagSkip::NoModel))
    }
}

/// The ports [`AutoTagService`] is built from.
#[derive(Clone)]
pub struct AutoTagPorts {
    /// Applies the chosen tags (and announces them).
    pub service: Arc<TagService>,
    /// The tags on offer.
    pub tags: Arc<dyn TagRepo>,
    /// Who tagged the meeting so far.
    pub applied: Arc<dyn MeetingTagRepo>,
    /// Titles, summaries and transcripts.
    pub digests: Arc<dyn MeetingDigestReader>,
    /// The Settings switch.
    pub switch: Arc<dyn AutoTagSwitch>,
    /// The LLM gateway (`tagging` route).
    pub gateway: Arc<Gateway>,
}

/// [`AutoTagger`] over the gateway's `tagging` route.
pub struct AutoTagService {
    ports: AutoTagPorts,
}

impl AutoTagService {
    /// Creates the service over its ports.
    pub fn new(ports: AutoTagPorts) -> Self {
        Self { ports }
    }

    /// True when the user put any tag on the meeting by hand.
    async fn tagged_by_user(&self, meeting: &MeetingId) -> Result<bool, TagError> {
        let applied = self.ports.applied.tags_of(meeting).await?;
        Ok(applied.iter().any(|tag| tag.applied_by == TagSource::User))
    }

    /// Every auto-applicable tag with up to [`MAX_EXAMPLES`] examples.
    async fn candidates(&self, meeting: &MeetingId) -> Result<Vec<Candidate>, TagError> {
        let mut candidates = Vec::new();
        for tag in self.ports.tags.tags().await? {
            if !tag.auto_apply {
                continue;
            }
            let examples = self
                .ports
                .digests
                .examples(&tag.id, meeting, MAX_EXAMPLES)
                .await?;
            candidates.push(Candidate { tag, examples });
        }
        Ok(candidates)
    }

    /// The meeting's summary, else its transcript's opening; `None` when
    /// neither has any text.
    async fn content(&self, digest: MeetingDigest) -> Result<Option<MeetingContent>, TagError> {
        if let Some(summary) = digest.summary.filter(|summary| !summary.trim().is_empty()) {
            return Ok(Some(MeetingContent {
                title: digest.title,
                text: summary,
                is_summary: true,
            }));
        }
        let opening = self
            .ports
            .digests
            .transcript_opening(&digest.meeting_id, CONTENT_CHARS)
            .await?;
        Ok((!opening.trim().is_empty()).then_some(MeetingContent {
            title: digest.title,
            text: opening,
            is_summary: false,
        }))
    }

    /// Applies `chosen` as the auto-tagger, unless the user tagged the
    /// meeting while the model was thinking.
    async fn apply(&self, meeting: &MeetingId, chosen: Vec<TagId>) -> Result<AutoTagged, TagError> {
        if self.tagged_by_user(meeting).await? {
            return Ok(AutoTagged::Skipped(AutoTagSkip::TaggedByUser));
        }
        for tag in &chosen {
            self.ports
                .service
                .apply(meeting, tag, TagSource::Ai)
                .await?;
        }
        log::info!(target: "tags", "auto-tagged meeting {meeting} with {} tags", chosen.len());
        Ok(AutoTagged::Applied(chosen))
    }
}

#[async_trait]
impl AutoTagger for AutoTagService {
    async fn auto_tag(&self, meeting: &MeetingId) -> Result<AutoTagged, TagError> {
        if !self.ports.switch.auto_tag_enabled().await {
            return Ok(AutoTagged::Skipped(AutoTagSkip::Disabled));
        }
        let Some(digest) = self.ports.digests.digest(meeting).await? else {
            return Ok(AutoTagged::Skipped(AutoTagSkip::MeetingGone));
        };
        if self.tagged_by_user(meeting).await? {
            return Ok(AutoTagged::Skipped(AutoTagSkip::TaggedByUser));
        }
        let candidates = self.candidates(meeting).await?;
        if candidates.is_empty() {
            return Ok(AutoTagged::Skipped(AutoTagSkip::NoTags));
        }
        let Some(content) = self.content(digest).await? else {
            return Ok(AutoTagged::Skipped(AutoTagSkip::NoContent));
        };
        let prompt = tagging_prompt(&candidates, &content);
        let answer: TaggingAnswer = self
            .ports
            .gateway
            .structured(Route::Tagging, prompt)
            .await?;
        self.apply(meeting, chosen_tags(&answer, &candidates)).await
    }
}
