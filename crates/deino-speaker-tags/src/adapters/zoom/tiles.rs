//! Zoom's video tiles, the user's own name, and who speaks by them (see
//! the parent module for what Zoom exposes).

use deino_domain::DisplayName;

use super::super::described::split_name;
use crate::label::Label;
use crate::reading::{Active, Reading};
use crate::tree::{AxNode, BUTTON_ROLE, Frame};

/// The role of a video tile.
const TILE_ROLE: &str = "AXTabGroup";

/// The role of a Participants panel row's name.
const TEXT_ROLE: &str = "AXStaticText";

/// Speaker View: the active speaker's tile is more than this many times
/// the area of every other tile (in a real capture, 1261×709 against a
/// 92×52 thumbnail).
const SPEAKER_VIEW_RATIO: u64 = 4;

/// The first part of the main window's account button.
const ACCOUNT_BUTTON_PREFIX: &str = "Zoom";

/// One video tile.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Tile {
    /// The participant's name, and whether it marks the user.
    pub(super) label: Label,
    /// Their audio is on.
    pub(super) unmuted: bool,
    /// Their video is on.
    pub(super) video: bool,
    /// Where the tile is, when it was read.
    pub(super) frame: Option<Frame>,
}

/// Every video tile in `window`, in order.
pub(super) fn tiles(window: &AxNode) -> Vec<Tile> {
    window
        .descendants()
        .filter(|node| node.is(TILE_ROLE))
        .filter_map(tile)
        .collect()
}

/// A tile from "Name, Computer audio unmuted[, Video off]"; an element
/// without an audio state is not a tile.
fn tile(node: &AxNode) -> Option<Tile> {
    let (name, rest) = split_name(node.description.as_deref()?)?;
    let states: Vec<String> = rest
        .split(',')
        .map(|state| state.trim().to_lowercase())
        .collect();
    if !states.iter().any(|state| state.contains("audio")) {
        return None;
    }
    Some(Tile {
        label: Label::parse(name)?,
        unmuted: states.iter().any(|state| state.ends_with("audio unmuted")),
        video: !states.iter().any(|state| state == "video off"),
        frame: node.frame,
    })
}

/// The user's own name: the main window's account button, else a
/// Participants panel row marked "(me)".
pub(super) fn own_name(window: &AxNode, context: &[AxNode]) -> Option<DisplayName> {
    let everywhere = || context.iter().chain([window]).flat_map(AxNode::descendants);
    everywhere()
        .find_map(account_name)
        .or_else(|| everywhere().find_map(listed_self))
}

/// "Zoom, Alex Chen, In a Zoom Meeting, Pro account" names Alex Chen.
fn account_name(node: &AxNode) -> Option<DisplayName> {
    if !node.is(BUTTON_ROLE) {
        return None;
    }
    let parts: Vec<&str> = node
        .description
        .as_deref()?
        .split(',')
        .map(str::trim)
        .collect();
    match parts.as_slice() {
        [ACCOUNT_BUTTON_PREFIX, name, .., last] if last.to_lowercase().ends_with("account") => {
            DisplayName::parse(name).ok()
        }
        _ => None,
    }
}

/// "Alex Chen (Host, me)" in the Participants panel names Alex Chen.
fn listed_self(node: &AxNode) -> Option<DisplayName> {
    if !node.is(TEXT_ROLE) {
        return None;
    }
    let text = node.text()?;
    let label = Label::parse(text).filter(|label| label.is_self && text.ends_with(')'))?;
    Some(label.name)
}

/// The reading the tiles give (see the parent module's rules).
pub(super) fn reading(tiles: &[Tile], own: Option<&DisplayName>) -> Reading {
    let is_self = |tile: &Tile| {
        tile.label.is_self || own.is_some_and(|own| own.key() == tile.label.name.key())
    };
    let mut participants: Vec<DisplayName> = Vec::new();
    for tile in tiles.iter().filter(|tile| !is_self(tile)) {
        if !participants.contains(&tile.label.name) {
            participants.push(tile.label.name.clone());
        }
    }
    let active = match speaker_view(tiles) {
        Some(large) if is_self(large) => Active::Me,
        Some(large) => Active::Speaking(large.label.name.clone()),
        None => {
            let open: Vec<&Tile> = tiles.iter().filter(|t| t.unmuted && !is_self(t)).collect();
            match open.as_slice() {
                [] => Active::Nobody,
                [only] => Active::Speaking(only.label.name.clone()),
                _ => Active::Several,
            }
        }
    };
    Reading {
        participants,
        active,
    }
}

/// Speaker View's large tile: one tile more than [`SPEAKER_VIEW_RATIO`]
/// times the area of every other. Needs at least two tiles, all framed.
fn speaker_view(tiles: &[Tile]) -> Option<&Tile> {
    if tiles.len() < 2 {
        return None;
    }
    let areas: Vec<u64> = tiles
        .iter()
        .map(|tile| tile.frame.map(|frame| frame.area()))
        .collect::<Option<_>>()?;
    let (largest, area) = areas
        .iter()
        .copied()
        .enumerate()
        .max_by_key(|&(_, area)| area)?;
    let dominant = areas
        .iter()
        .enumerate()
        .all(|(index, other)| index == largest || other.saturating_mul(SPEAKER_VIEW_RATIO) < area);
    dominant.then(|| tiles.get(largest)).flatten()
}

#[cfg(test)]
#[path = "tiles_tests.rs"]
mod tests;
