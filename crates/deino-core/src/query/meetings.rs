//! [`MeetingQueries`]: listing meetings by date, tag, folder, person and
//! company, and reading one in full.

use std::collections::HashSet;

use chrono::NaiveDate;
use deino_domain::{CompanyId, FolderId, MeetingId, Seconds};

use super::action_items::summarize;
use super::citations::citations;
use super::link::meeting_link;
use super::{
    AttendeeView, AudioFiles, MeetingDetail, MeetingSummary, QueryError, ReadPorts, Resolver,
};
use crate::action_items::ActionItemFilter;
use crate::export::ExportAssembler;
use crate::meetings::{Meeting, MeetingState};

/// How many meetings a list returns when the caller does not say.
pub const DEFAULT_MEETINGS: u32 = 20;
/// The most meetings one list returns.
pub const MAX_MEETINGS: u32 = 200;

/// Which meetings to list, newest first. Every condition narrows the list;
/// tag, folder, person and company are IDs or names ([`Resolver`]).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MeetingFilter {
    /// The most meetings to return; 0 means [`DEFAULT_MEETINGS`].
    pub limit: u32,
    /// The first local day, inclusive.
    pub from: Option<NaiveDate>,
    /// The last local day, inclusive.
    pub to: Option<NaiveDate>,
    /// Only meetings with this tag.
    pub tag: Option<String>,
    /// Only meetings in this folder or a folder inside it.
    pub folder: Option<String>,
    /// Only meetings this person attended.
    pub person: Option<String>,
    /// Only meetings someone from this company attended.
    pub company: Option<String>,
}

impl MeetingFilter {
    fn limit(&self) -> usize {
        let limit = match self.limit {
            0 => DEFAULT_MEETINGS,
            limit => limit.min(MAX_MEETINGS),
        };
        usize::try_from(limit).unwrap_or(usize::MAX)
    }

    fn admits(&self, day: NaiveDate) -> bool {
        self.from.is_none_or(|from| day >= from) && self.to.is_none_or(|to| day <= to)
    }
}

/// Reads meetings.
pub struct MeetingQueries {
    ports: ReadPorts,
    assembler: ExportAssembler,
}

impl MeetingQueries {
    /// The queries over `ports`.
    pub fn new(ports: ReadPorts) -> Self {
        let assembler = ExportAssembler::new(ports.meeting.clone());
        Self { ports, assembler }
    }

    /// The meetings `filter` matches, newest first, outside the trash.
    pub async fn list(&self, filter: &MeetingFilter) -> Result<Vec<MeetingSummary>, QueryError> {
        if filter
            .from
            .zip(filter.to)
            .is_some_and(|(from, to)| from > to)
        {
            return Err(QueryError::InvalidRange);
        }
        let scope = self.scope(filter).await?;
        let mut found = Vec::new();
        for meeting in self.ports.meeting.meetings.list().await? {
            if found.len() == filter.limit() {
                break;
            }
            if scope.as_ref().is_some_and(|ids| !ids.contains(&meeting.id)) {
                continue;
            }
            let state = self.state(&meeting.id).await?;
            let summary = self.summary(meeting, &state).await?;
            if filter.admits(summary.held_at.date_naive()) {
                found.push(summary);
            }
        }
        Ok(found)
    }

    /// The meeting with `id`, everything but its transcript.
    pub async fn read(&self, id: &MeetingId) -> Result<MeetingDetail, QueryError> {
        let ports = &self.ports.meeting;
        let meeting = ports
            .meetings
            .find(id)
            .await?
            .ok_or_else(|| QueryError::meeting_not_found(id))?;
        let state = self.state(id).await?;
        let notes = ports.notes.notes(id).await?;
        let enhanced = self.ports.enhanced.latest(id).await?;
        Ok(MeetingDetail {
            people: self.people(id).await?,
            folders: names(ports.folders.folders_of(id).await?, |folder| folder.name),
            my_notes: notes.mine,
            ai_notes: notes.enhanced,
            ai_notes_version: enhanced.as_ref().map(|notes| notes.version),
            citations: enhanced.map_or_else(Vec::new, |notes| citations(&notes.doc, id)),
            action_items: self.action_items(id).await?,
            audio: audio_files(&state),
            transcript_segments: ports.segments.count(id).await?,
            summary: self.summary(meeting, &state).await?,
        })
    }

    async fn state(&self, id: &MeetingId) -> Result<MeetingState, QueryError> {
        self.ports
            .meeting
            .lifecycle
            .state(id)
            .await?
            .ok_or_else(|| QueryError::meeting_not_found(id))
    }

    async fn summary(
        &self,
        meeting: Meeting,
        state: &MeetingState,
    ) -> Result<MeetingSummary, QueryError> {
        let ports = &self.ports.meeting;
        let started = state.started_at.unwrap_or(meeting.created_at);
        let attendees = self.assembler.attendees(&meeting.id).await?;
        let tags = ports.tags.tags_of(&meeting.id).await?;
        Ok(MeetingSummary {
            link: meeting_link(&meeting.id),
            held_at: ports.zone.local(started),
            duration_s: state.duration.map(Seconds::get),
            status: state.status,
            attendees: names(attendees, |attendee| attendee.name),
            tags: names(tags, |applied| applied.tag.name),
            title: meeting.title,
            id: meeting.id,
        })
    }

    async fn people(&self, id: &MeetingId) -> Result<Vec<AttendeeView>, QueryError> {
        let attendees = self.assembler.attendees(id).await?;
        Ok(attendees
            .into_iter()
            .map(|attendee| AttendeeView {
                name: attendee.name,
                email: attendee.email,
                is_me: attendee.is_me,
            })
            .collect())
    }

    async fn action_items(
        &self,
        id: &MeetingId,
    ) -> Result<Vec<super::ActionItemSummary>, QueryError> {
        let filter = ActionItemFilter::meeting(id.clone());
        let views = self.ports.action_items.list(&filter).await?;
        let zone = self.ports.meeting.zone.as_ref();
        Ok(views
            .into_iter()
            .map(|view| summarize(view, zone))
            .collect())
    }

    /// The meetings the filter's references allow; `None` when none is set.
    async fn scope(
        &self,
        filter: &MeetingFilter,
    ) -> Result<Option<HashSet<MeetingId>>, QueryError> {
        let resolver = Resolver::new(&self.ports);
        let mut sets = Vec::new();
        if let Some(tag) = &filter.tag {
            let tag = resolver.tag(tag).await?;
            sets.push(ids(self.ports.tagging.meetings_tagged(&tag).await?));
        }
        if let Some(folder) = &filter.folder {
            sets.push(self.in_folder(resolver.folder(folder).await?).await?);
        }
        if let Some(person) = &filter.person {
            let person = resolver.person(person).await?;
            sets.push(ids(self
                .ports
                .meeting
                .attendees
                .meetings_of(&person)
                .await?));
        }
        if let Some(company) = &filter.company {
            sets.push(self.with_company(resolver.company(company).await?).await?);
        }
        Ok(sets.into_iter().reduce(|kept, next| &kept & &next))
    }

    /// Meetings in `root` or any folder inside it.
    async fn in_folder(&self, root: FolderId) -> Result<HashSet<MeetingId>, QueryError> {
        let folders = self.ports.folders.folders().await?;
        let mut tree = vec![root];
        let mut index = 0;
        while let Some(parent) = tree.get(index).copied() {
            let children = folders
                .iter()
                .filter(|folder| folder.parent_id == Some(parent));
            tree.extend(children.map(|folder| folder.id));
            index += 1;
        }
        let mut meetings = HashSet::new();
        for folder in tree {
            meetings.extend(ids(self.ports.meeting.folders.meetings_in(&folder).await?));
        }
        Ok(meetings)
    }

    /// Meetings someone from `company` attended.
    async fn with_company(&self, company: CompanyId) -> Result<HashSet<MeetingId>, QueryError> {
        let people = self.ports.meeting.people.list_by_company(&company).await?;
        let mut meetings = HashSet::new();
        for person in people {
            meetings.extend(ids(self
                .ports
                .meeting
                .attendees
                .meetings_of(&person.id)
                .await?));
        }
        Ok(meetings)
    }
}

fn ids(meetings: Vec<Meeting>) -> HashSet<MeetingId> {
    meetings.into_iter().map(|meeting| meeting.id).collect()
}

fn names<T>(items: Vec<T>, name: impl Fn(T) -> String) -> Vec<String> {
    items.into_iter().map(name).collect()
}

fn audio_files(state: &MeetingState) -> AudioFiles {
    AudioFiles {
        mixed: state.audio.audio_path.clone(),
        microphone: state.audio.audio_mic_path.clone(),
        system: state.audio.audio_system_path.clone(),
    }
}

#[cfg(test)]
#[path = "meetings_tests.rs"]
mod tests;
