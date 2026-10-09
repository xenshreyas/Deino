//! [`TagService`]: tags and the meetings they label.
//!
//! Tag names are unique ignoring case; a new tag gets the next palette
//! colour unless the user picks one. Applying a tag records who applied it
//! and announces it ([`TagEvents`]), so automations can react.

use std::sync::Arc;

use deino_domain::{Clock, MeetingId, PaletteColor, TagId, TagSource};

use super::{
    AppliedTag, MeetingTagLink, MeetingTagRepo, MeetingTagged, NewTag, Tag, TagApplication,
    TagDescription, TagError, TagEvents, TagName, TagRepo, TagUpdate,
};

/// The ports [`TagService`] is built from.
#[derive(Clone)]
pub struct TagPorts {
    /// The tags.
    pub tags: Arc<dyn TagRepo>,
    /// Which tags sit on which meetings.
    pub applied: Arc<dyn MeetingTagRepo>,
    /// Where `MeetingTagged` goes.
    pub events: Arc<dyn TagEvents>,
    /// The clock that stamps changes.
    pub clock: Arc<dyn Clock>,
}

/// Creates, edits and deletes tags, and puts them on meetings.
pub struct TagService {
    ports: TagPorts,
}

impl TagService {
    /// Creates the service over its ports.
    pub fn new(ports: TagPorts) -> Self {
        Self { ports }
    }

    /// Every tag, by name.
    pub async fn list(&self) -> Result<Vec<Tag>, TagError> {
        Ok(self.ports.tags.tags().await?)
    }

    /// Creates a tag; it may be applied automatically from the start.
    pub async fn create(&self, new: &NewTag) -> Result<Tag, TagError> {
        let name = TagName::parse(&new.name)?;
        let description = TagDescription::parse(&new.description)?;
        if self.ports.tags.tag_by_name(name.as_str()).await?.is_some() {
            return Err(TagError::NameTaken);
        }
        let color = match new.color {
            Some(color) => color,
            None => PaletteColor::cycle(self.ports.tags.tags().await?.len()),
        };
        let now = self.ports.clock.now();
        let tag = Tag {
            id: TagId::generate(self.ports.clock.as_ref()),
            name: name.as_str().to_owned(),
            color,
            description: description.as_str().to_owned(),
            auto_apply: true,
            created_at: now,
            updated_at: now,
        };
        self.ports
            .tags
            .create_tag(&tag)
            .await
            .map_err(TagError::naming)?;
        log::info!(target: "tags", "created tag {}", tag.id);
        Ok(tag)
    }

    /// Replaces the tag's name, colour, description and auto-apply switch.
    pub async fn update(&self, id: &TagId, update: &TagUpdate) -> Result<Tag, TagError> {
        let name = TagName::parse(&update.name)?;
        let description = TagDescription::parse(&update.description)?;
        let tag = self.get(id).await?;
        let clash = self.ports.tags.tag_by_name(name.as_str()).await?;
        if clash.is_some_and(|other| other.id != *id) {
            return Err(TagError::NameTaken);
        }
        let updated = Tag {
            name: name.as_str().to_owned(),
            color: update.color,
            description: description.as_str().to_owned(),
            auto_apply: update.auto_apply,
            updated_at: self.ports.clock.now(),
            ..tag
        };
        if !self
            .ports
            .tags
            .update_tag(&updated)
            .await
            .map_err(TagError::naming)?
        {
            return Err(TagError::NotFound(*id));
        }
        Ok(updated)
    }

    /// Deletes the tag; it leaves every meeting (and every folder rule).
    pub async fn delete(&self, id: &TagId) -> Result<(), TagError> {
        if !self.ports.tags.delete_tag(id).await? {
            return Err(TagError::NotFound(*id));
        }
        log::info!(target: "tags", "deleted tag {id}");
        Ok(())
    }

    /// The tags on the meeting, by name, with who applied them.
    pub async fn tags_of(&self, meeting: &MeetingId) -> Result<Vec<AppliedTag>, TagError> {
        Ok(self.ports.applied.tags_of(meeting).await?)
    }

    /// Every tag on every meeting, for the Meetings page.
    pub async fn links(&self) -> Result<Vec<MeetingTagLink>, TagError> {
        Ok(self.ports.applied.links().await?)
    }

    /// Puts the tag on the meeting as `source` and announces it if it is new.
    /// Returns true when the meeting did not have the tag yet.
    pub async fn apply(
        &self,
        meeting: &MeetingId,
        tag: &TagId,
        source: TagSource,
    ) -> Result<bool, TagError> {
        self.get(tag).await?;
        let application = TagApplication {
            meeting_id: meeting.clone(),
            tag_id: *tag,
            applied_by: source,
            applied_at: self.ports.clock.now(),
        };
        let added = self
            .ports
            .applied
            .apply_tag(&application)
            .await
            .map_err(|error| {
                if error.is_foreign_key_violation() {
                    TagError::MeetingNotFound(meeting.clone())
                } else {
                    TagError::Storage(error)
                }
            })?;
        if added {
            self.announce(application);
        }
        Ok(added)
    }

    /// Takes the tag off the meeting; taking off a tag it does not have is
    /// not an error.
    pub async fn remove(&self, meeting: &MeetingId, tag: &TagId) -> Result<(), TagError> {
        self.ports.applied.remove_tag(meeting, tag).await?;
        Ok(())
    }

    async fn get(&self, id: &TagId) -> Result<Tag, TagError> {
        self.ports
            .tags
            .tag(id)
            .await?
            .ok_or(TagError::NotFound(*id))
    }

    fn announce(&self, application: TagApplication) {
        log::info!(
            target: "tags",
            "tagged meeting {} with tag {} ({})",
            application.meeting_id,
            application.tag_id,
            application.applied_by
        );
        self.ports.events.meeting_tagged(&MeetingTagged {
            meeting_id: application.meeting_id,
            tag_id: application.tag_id,
            applied_by: application.applied_by,
        });
    }
}
