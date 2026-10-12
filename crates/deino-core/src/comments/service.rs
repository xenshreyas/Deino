//! [`CommentService`]: starting, answering, editing, resolving and deleting
//! comment threads.

use std::sync::Arc;

use deino_domain::{Clock, CommentId, CommentThreadId, MeetingId};

use super::{
    Comment, CommentAnchor, CommentError, CommentRepo, CommentTarget, CommentThread, NewThread,
    ThreadRow,
};

/// The user's private comments on transcripts and notes.
pub struct CommentService {
    repo: Arc<dyn CommentRepo>,
    clock: Arc<dyn Clock>,
}

impl CommentService {
    /// The service over its table and clock.
    pub fn new(repo: Arc<dyn CommentRepo>, clock: Arc<dyn Clock>) -> Self {
        Self { repo, clock }
    }

    /// A meeting's threads, oldest first.
    pub async fn list(&self, meeting: &MeetingId) -> Result<Vec<CommentThread>, CommentError> {
        Ok(self.repo.list(meeting).await?)
    }

    /// Starts a thread on a span with its first comment.
    pub async fn start(&self, new: NewThread) -> Result<CommentThread, CommentError> {
        check_anchor(&new.anchor)?;
        let id = CommentThreadId::generate(self.clock.as_ref());
        let first = self.comment(id, &new.body)?;
        let row = ThreadRow {
            id,
            meeting_id: new.meeting_id,
            anchor: new.anchor,
            first,
        };
        if !self.repo.insert_thread(&row).await? {
            return Err(CommentError::InvalidAnchor);
        }
        self.thread(&row.meeting_id, id).await
    }

    /// Adds a reply to a thread.
    pub async fn reply(
        &self,
        thread: CommentThreadId,
        body: &str,
    ) -> Result<Comment, CommentError> {
        let comment = self.comment(thread, body)?;
        if self.repo.insert_comment(&comment).await? {
            Ok(comment)
        } else {
            Err(CommentError::NotFound)
        }
    }

    /// Replaces a comment's text.
    pub async fn edit(&self, id: CommentId, body: &str) -> Result<(), CommentError> {
        let body = checked_body(body)?;
        found(self.repo.edit(&id, &body, self.clock.now()).await?)
    }

    /// Resolves the thread, or reopens it.
    pub async fn set_resolved(
        &self,
        thread: &CommentThreadId,
        resolved: bool,
    ) -> Result<(), CommentError> {
        let at = resolved.then(|| self.clock.now());
        found(self.repo.set_resolved(thread, at).await?)
    }

    /// Deletes a comment; deleting a thread's last comment deletes the thread.
    pub async fn delete(&self, id: &CommentId) -> Result<(), CommentError> {
        found(self.repo.delete(id).await?)
    }

    /// A new comment of `thread`, stamped now.
    fn comment(&self, thread: CommentThreadId, body: &str) -> Result<Comment, CommentError> {
        let now = self.clock.now();
        Ok(Comment {
            id: CommentId::generate(self.clock.as_ref()),
            thread_id: thread,
            body: checked_body(body)?,
            created_at: now,
            updated_at: now,
        })
    }

    /// The thread just saved.
    async fn thread(
        &self,
        meeting: &MeetingId,
        id: CommentThreadId,
    ) -> Result<CommentThread, CommentError> {
        let threads = self.repo.list(meeting).await?;
        threads
            .into_iter()
            .find(|thread| thread.id == id)
            .ok_or(CommentError::NotFound)
    }
}

/// The span is not empty, and a line is named exactly for transcript threads.
fn check_anchor(anchor: &CommentAnchor) -> Result<(), CommentError> {
    let has_line = anchor.segment_id.is_some();
    let fits = match anchor.target {
        CommentTarget::Transcript => has_line,
        CommentTarget::Notes | CommentTarget::Enhanced => !has_line,
    };
    if fits && anchor.end > anchor.start && !anchor.quote.is_empty() {
        Ok(())
    } else {
        Err(CommentError::InvalidAnchor)
    }
}

/// The trimmed body, or [`CommentError::EmptyBody`].
fn checked_body(body: &str) -> Result<String, CommentError> {
    let body = body.trim();
    if body.is_empty() {
        Err(CommentError::EmptyBody)
    } else {
        Ok(body.to_owned())
    }
}

/// `Ok` when the row was there, else [`CommentError::NotFound`].
fn found(changed: bool) -> Result<(), CommentError> {
    if changed {
        Ok(())
    } else {
        Err(CommentError::NotFound)
    }
}
