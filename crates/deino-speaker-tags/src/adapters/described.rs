//! The scanner for apps that describe a participant in one accessibility
//! description: the name, then comma-separated states,
//! "Alex Chen (Host), Computer audio unmuted, Video on, Talking".
//!
//! An element is a participant when its role is one the app uses for tiles
//! and roster rows and at least one state is known. Audio states ("muted",
//! "unmuted") mean the app shows who speaks, so a window with them and none
//! speaking is silent rather than unreadable; speaking states light the
//! element's indicator.

use crate::label::Label;
use crate::reading::Scan;
use crate::tree::AxNode;

/// One app's way of describing participants. Every word is matched,
/// lower-case, as a substring of one comma-separated state.
#[derive(Clone, Copy, Debug)]
pub(super) struct Grammar {
    /// The roles of tiles and roster rows.
    pub(super) roles: &'static [&'static str],
    /// States that only say the element is a participant (video, role).
    pub(super) presence: &'static [&'static str],
    /// Audio states: the element shows whether its participant speaks.
    pub(super) audio: &'static [&'static str],
    /// States that mean the participant is speaking now.
    pub(super) speaking: &'static [&'static str],
}

/// What one element's description says.
struct Described {
    label: Label,
    audio: bool,
    speaking: bool,
}

/// Every participant element under `window`.
pub(super) fn scan(window: &AxNode, grammar: &Grammar) -> Scan {
    let mut scan = Scan::default();
    for described in window
        .descendants()
        .filter(|node| grammar.roles.contains(&node.role.as_str()))
        .filter_map(|node| describe(node, grammar))
    {
        if described.audio || described.speaking {
            scan.indicators += 1;
        }
        if described.speaking {
            scan.lit.push(described.label.clone());
        }
        scan.labels.push(described.label);
    }
    scan
}

/// The element's name and states, if it describes a participant.
fn describe(node: &AxNode, grammar: &Grammar) -> Option<Described> {
    let text = node.description.as_deref().or(node.title.as_deref())?;
    let mut parts = text.split(',');
    let name = parts.next()?;
    let states: Vec<String> = parts.map(|part| part.trim().to_lowercase()).collect();
    let has = |words: &[&str]| {
        states
            .iter()
            .any(|state| words.iter().any(|word| state.contains(word)))
    };
    // "Speaking" must open the state, so "not speaking" never lights it.
    let speaking = states
        .iter()
        .any(|state| grammar.speaking.iter().any(|word| state.starts_with(word)));
    let audio = has(grammar.audio);
    if !(audio || speaking || has(grammar.presence)) {
        return None;
    }
    Some(Described {
        label: Label::parse(name)?,
        audio,
        speaking,
    })
}
