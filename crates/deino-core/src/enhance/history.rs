//! Reading, editing and restoring versions of a meeting's enhanced notes,
//! and the transcript excerpts their citations point at.

use deino_doc::{Doc, Flavor};
use deino_domain::{MeetingId, SegmentId};

use super::{
    CitedSegment, EnhanceError, EnhanceService, EnhancedNotes, EnhancedNotesView, NewVersion,
    SavedEnhancedNotes, VersionEdit, VersionInfo,
};

/// The most excerpts one popover asks for.
pub const MAX_CITED: usize = 12;

impl EnhanceService {
    /// Version `version` of the meeting's notes (the newest when `None`),
    /// with the regenerate reasons when it is the newest; `None` before the
    /// first version.
    pub async fn view(
        &self,
        meeting: &MeetingId,
        version: Option<u32>,
    ) -> Result<Option<EnhancedNotesView>, EnhanceError> {
        let latest = self.ports.reader.latest(meeting).await?;
        let Some(latest) = latest else {
            return Ok(None);
        };
        let newest = latest.version;
        let notes = match version {
            Some(number) if number != newest => {
                self.ports
                    .reader
                    .version(meeting, number)
                    .await?
                    .ok_or(EnhanceError::VersionNotFound { version: number })?
            }
            _ => latest,
        };
        let is_latest = notes.version == newest;
        let reasons = match (&notes.basis, is_latest) {
            (Some(basis), true) => basis.reasons(&self.material(meeting).await?),
            _ => Vec::new(),
        };
        let info = self.info(meeting, notes.version).await?;
        Ok(Some(EnhancedNotesView {
            meeting_id: meeting.clone(),
            version: info,
            is_latest,
            doc: notes.doc,
            reasons,
        }))
    }

    /// Every version of the meeting's notes, newest first.
    pub async fn versions(&self, meeting: &MeetingId) -> Result<Vec<VersionInfo>, EnhanceError> {
        Ok(self.ports.reader.versions(meeting).await?)
    }

    /// Saves the user's edit of the newest version and marks it edited.
    pub async fn save(
        &self,
        meeting: &MeetingId,
        version: u32,
        doc: Doc,
    ) -> Result<SavedEnhancedNotes, EnhanceError> {
        let doc = doc.normalized();
        deino_doc::validate(&doc).map_err(EnhanceError::InvalidDoc)?;
        let edit = VersionEdit {
            meeting_id: meeting.clone(),
            version,
            markdown: deino_doc::to_markdown(&doc, Flavor::Readable),
            doc,
            at: self.clock.now(),
        };
        if self.ports.writer.save_edit(&edit).await? {
            log::debug!(target: "enhance", "{meeting}: saved an edit of version {version}");
            return Ok(SavedEnhancedNotes {
                meeting_id: meeting.clone(),
                version,
                updated_at: edit.at,
            });
        }
        match self.ports.reader.version(meeting, version).await? {
            Some(_) => Err(EnhanceError::NotLatest),
            None => Err(EnhanceError::VersionNotFound { version }),
        }
    }

    /// Makes a copy of an earlier version the newest one, edits and all.
    pub async fn restore(
        &self,
        meeting: &MeetingId,
        version: u32,
    ) -> Result<EnhancedNotesView, EnhanceError> {
        let source = self
            .ports
            .reader
            .version(meeting, version)
            .await?
            .ok_or(EnhanceError::VersionNotFound { version })?;
        let restored = self
            .ports
            .writer
            .insert(&copy_of(source, self.clock.now()))
            .await?;
        log::info!(target: "enhance", "{meeting}: restored version {version} as {restored}");
        self.view(meeting, Some(restored))
            .await?
            .ok_or(EnhanceError::VersionNotFound { version: restored })
    }

    /// The transcript excerpts behind citations of `segments`, in time
    /// order; segments deleted since are left out.
    pub async fn cited(
        &self,
        meeting: &MeetingId,
        segments: &[SegmentId],
    ) -> Result<Vec<CitedSegment>, EnhanceError> {
        let wanted: Vec<&SegmentId> = segments.iter().take(MAX_CITED).collect();
        let lines = self.ports.sources.transcript_of(meeting).await?;
        Ok(lines
            .into_iter()
            .filter(|line| wanted.contains(&&line.segment_id))
            .map(|line| CitedSegment {
                segment_id: line.segment_id,
                start: line.start,
                end: line.end,
                speaker: line.speaker,
                text: line.text,
            })
            .collect())
    }

    async fn info(&self, meeting: &MeetingId, version: u32) -> Result<VersionInfo, EnhanceError> {
        self.ports
            .reader
            .versions(meeting)
            .await?
            .into_iter()
            .find(|info| info.version == version)
            .ok_or(EnhanceError::VersionNotFound { version })
    }
}

fn copy_of(source: EnhancedNotes, at: chrono::DateTime<chrono::Utc>) -> NewVersion {
    NewVersion {
        meeting_id: source.meeting_id,
        doc: source.doc,
        markdown: source.markdown,
        template_id: source.template_id,
        model: source.model,
        basis: source.basis,
        is_user_edited: source.is_user_edited,
        at,
    }
}
