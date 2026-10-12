//! [`FollowUpService`]: what the `followups` plugin calls. Home's
//! suggestions, the meeting's draft, the user's edits, dismissing (for
//! good), and the two ways to send while Gmail and Outlook wait for INT-07:
//! Mail through a `mailto:` link, or the clipboard.

use std::sync::Arc;

use chrono::TimeDelta;
use deino_domain::{Clock, Email, MeetingId};
use deino_export::mailto::{Compose, compose};

use super::{
    FollowUpDraft, FollowUpEdit, FollowUpError, FollowUpRepo, FollowUpStatus, FollowUpSummary,
    Recipients, StoredFollowUp,
};
use crate::export::{Clipboard, ClipboardContent, UrlOpener};

/// How long a ready draft is suggested on Home.
pub const SHOWN_ON_HOME: TimeDelta = TimeDelta::days(3);
/// The longest `mailto:` link handed to Mail; longer drafts are copied.
const MAX_MAILTO_CHARS: usize = 32_000;

/// The ports [`FollowUpService`] reads and acts through.
pub struct FollowUpPorts {
    /// Stored follow-ups.
    pub store: Arc<dyn FollowUpRepo>,
    /// Opens `mailto:` links in the user's mail app.
    pub opener: Arc<dyn UrlOpener>,
    /// The system clipboard.
    pub clipboard: Arc<dyn Clipboard>,
    /// "Now", for Home's window and edit times.
    pub clock: Arc<dyn Clock>,
}

/// Reads, edits and sends suggested follow-ups.
pub struct FollowUpService {
    ports: FollowUpPorts,
}

impl FollowUpService {
    /// The service over `ports`.
    pub fn new(ports: FollowUpPorts) -> Self {
        Self { ports }
    }

    /// Home's "Suggested follow-ups": ready drafts of the last three days,
    /// newest first.
    pub async fn suggested(&self) -> Result<Vec<FollowUpSummary>, FollowUpError> {
        let since = self.ports.clock.now() - SHOWN_ON_HOME;
        Ok(self.ports.store.ready_since(since).await?)
    }

    /// The meeting's ready draft, or `None`.
    pub async fn for_meeting(
        &self,
        meeting: &MeetingId,
    ) -> Result<Option<FollowUpDraft>, FollowUpError> {
        let stored = self.ports.store.find(meeting).await?;
        Ok(stored.and_then(StoredFollowUp::draft))
    }

    /// Stores the user's edit of the meeting's draft and returns it. Every
    /// address must be valid.
    pub async fn save(
        &self,
        meeting: &MeetingId,
        edit: FollowUpEdit,
    ) -> Result<FollowUpDraft, FollowUpError> {
        let mut stored = self
            .ports
            .store
            .find(meeting)
            .await?
            .filter(|stored| stored.status == FollowUpStatus::Ready)
            .ok_or_else(|| FollowUpError::NoDraft(meeting.clone()))?;
        stored.recipients = recipients(&edit)?;
        stored.subject = edit.subject;
        stored.body = edit.body;
        stored.edited_at = Some(self.ports.clock.now());
        if !self.ports.store.edit(&stored).await? {
            return Err(FollowUpError::NoDraft(meeting.clone()));
        }
        stored
            .draft()
            .ok_or_else(|| FollowUpError::NoDraft(meeting.clone()))
    }

    /// Dismisses the meeting's follow-up for good.
    pub async fn dismiss(&self, meeting: &MeetingId) -> Result<(), FollowUpError> {
        let now = self.ports.clock.now();
        if self.ports.store.dismiss(meeting, now).await? {
            log::info!(target: "followups", "follow-up for meeting {meeting} dismissed");
        }
        Ok(())
    }

    /// Saves `edit`, then opens it in the user's mail app as a `mailto:`
    /// draft.
    pub async fn open_in_mail(
        &self,
        meeting: &MeetingId,
        edit: FollowUpEdit,
    ) -> Result<(), FollowUpError> {
        let draft = self.save(meeting, edit).await?;
        let url = mailto(&draft)?;
        Ok(self.ports.opener.open(&url).await?)
    }

    /// Saves `edit`, then copies its body to the clipboard.
    pub async fn copy(&self, meeting: &MeetingId, edit: FollowUpEdit) -> Result<(), FollowUpError> {
        let draft = self.save(meeting, edit).await?;
        let content = ClipboardContent {
            text: draft.body,
            html: None,
        };
        Ok(self.ports.clipboard.write(content).await?)
    }
}

/// The edit's addresses, each checked, blanks dropped.
fn recipients(edit: &FollowUpEdit) -> Result<Recipients, FollowUpError> {
    let parse = |list: &[String]| -> Result<Vec<Email>, FollowUpError> {
        list.iter()
            .map(|address| address.trim())
            .filter(|address| !address.is_empty())
            .map(|address| Email::parse(address).map_err(|_| FollowUpError::InvalidRecipient))
            .collect()
    };
    Ok(Recipients {
        to: parse(&edit.to)?,
        cc: parse(&edit.cc)?,
        bcc: parse(&edit.bcc)?,
    })
}

/// The addresses as text.
fn addresses(list: &[Email]) -> Vec<&str> {
    list.iter().map(Email::as_str).collect()
}

/// The draft as a `mailto:` link; it needs a recipient and must fit.
fn mailto(draft: &FollowUpDraft) -> Result<String, FollowUpError> {
    let all = &draft.recipients;
    if all.to.is_empty() && all.cc.is_empty() && all.bcc.is_empty() {
        return Err(FollowUpError::NoRecipients);
    }
    let strs = addresses;
    let (to, cc, bcc) = (strs(&all.to), strs(&all.cc), strs(&all.bcc));
    let url = compose(&Compose {
        to: &to,
        cc: &cc,
        bcc: &bcc,
        subject: &draft.subject,
        body: &draft.body,
    });
    if url.len() > MAX_MAILTO_CHARS {
        return Err(FollowUpError::TooLongForMail);
    }
    Ok(url)
}
