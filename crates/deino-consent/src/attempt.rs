//! One attempt to post the notice: [`post`], over the driver and the key
//! activity, re-checking the typing guard between every step.
//!
//! ```text
//!   quiet? ─▶ composer empty? ─▶ focus ─▶ quiet? ─▶ write ─▶ reads back as
//!   the notice? ─▶ quiet? ─▶ press Send ─▶ composer emptied? ─▶ Sent
//! ```
//!
//! Before anything is written, a failed check just waits for a later poll.
//! After, Deino takes its own text back out (only when the composer holds
//! exactly the notice) and waits; when the composer holds anything else,
//! the user typed into it, so Deino leaves it alone and gives up for the
//! meeting. Nothing here posts key events: text goes in through `AXValue`
//! and the message goes out through the send button's `AXPress`.

use crate::error::ChatError;
use crate::gate::{Hold, is_quiet};
use crate::ports::{ChatDriver, KeyActivity, Target};

/// One post: the ports, the composer and its send button, the message.
pub struct Post<'a> {
    /// Acts on the chat.
    pub driver: &'a dyn ChatDriver,
    /// The typing guard's source.
    pub keys: &'a dyn KeyActivity,
    /// The composer.
    pub field: Target,
    /// Its send button.
    pub send: Target,
    /// The notice.
    pub message: &'a str,
}

/// Why an attempt will be tried again.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Retry {
    /// A check failed before anything was left behind.
    Hold(Hold),
    /// The driver failed before anything was left behind.
    Failed(ChatError),
}

/// How an attempt ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// The notice was posted.
    Sent,
    /// Nothing was posted or left behind; try again on a later poll.
    Retry(Retry),
    /// Stop for this meeting: retrying could post twice or overwrite the user.
    GaveUp(ChatError),
}

/// Posts `post.message` once, or explains why not.
pub fn post(post: &Post<'_>) -> Outcome {
    match write(post) {
        Ok(()) => send(post),
        Err(outcome) => outcome,
    }
}

/// Writes the notice into the empty composer and checks it took.
fn write(post: &Post<'_>) -> Result<(), Outcome> {
    let driver = post.driver;
    guard(post)?;
    let before = driver.value(&post.field).map_err(failed)?;
    if !before.trim().is_empty() {
        return Err(Outcome::Retry(Retry::Hold(Hold::Drafting)));
    }
    driver.focus(&post.field).map_err(failed)?;
    guard(post)?;
    if let Err(error) = driver.set_value(&post.field, post.message) {
        return Err(undo(post, Retry::Failed(error)));
    }
    match driver.value(&post.field) {
        Ok(now) if now == post.message => Ok(()),
        Ok(now) if now.trim().is_empty() => {
            Err(Outcome::GaveUp(ChatError::NotTyped(post.field.app)))
        }
        Ok(_) => Err(Outcome::GaveUp(ChatError::Interrupted(post.field.app))),
        Err(error) => Err(undo(post, Retry::Failed(error))),
    }
}

/// Presses Send if the user is still quiet, and checks the composer emptied.
fn send(post: &Post<'_>) -> Outcome {
    if !is_quiet(post.keys.since_last_key()) {
        return undo(post, Retry::Hold(Hold::Typing));
    }
    let pressed = post.driver.press(&post.send);
    match (pressed, post.driver.value(&post.field)) {
        (_, Ok(now)) if now.trim().is_empty() => Outcome::Sent,
        (Ok(()), _) => give_up(post, ChatError::NotSent(post.field.app)),
        (Err(error), _) => give_up(post, error),
    }
}

/// The typing guard: fails when a key was pressed less than `QUIET` ago.
fn guard(post: &Post<'_>) -> Result<(), Outcome> {
    if is_quiet(post.keys.since_last_key()) {
        Ok(())
    } else {
        Err(Outcome::Retry(Retry::Hold(Hold::Typing)))
    }
}

fn failed(error: ChatError) -> Outcome {
    Outcome::Retry(Retry::Failed(error))
}

/// Takes the notice back out of the composer, then retries with `then`.
/// Anything other than exactly the notice (or nothing) is the user's: left
/// alone, and the meeting is given up.
fn undo(post: &Post<'_>, then: Retry) -> Outcome {
    match post.driver.value(&post.field) {
        Ok(now) if now.trim().is_empty() => Outcome::Retry(then),
        Ok(now) if now == post.message => match post.driver.set_value(&post.field, "") {
            Ok(()) => Outcome::Retry(then),
            Err(error) => Outcome::GaveUp(error),
        },
        Ok(_) | Err(_) => Outcome::GaveUp(ChatError::Interrupted(post.field.app)),
    }
}

/// Takes the notice back out (if it is still there unchanged) and gives up.
fn give_up(post: &Post<'_>, error: ChatError) -> Outcome {
    if let Ok(now) = post.driver.value(&post.field)
        && now == post.message
    {
        post.driver.set_value(&post.field, "").ok();
    }
    Outcome::GaveUp(error)
}
