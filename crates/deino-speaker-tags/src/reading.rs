//! [`Reading`]: what one look at a meeting window says, and the typed ways
//! a look can fail ([`ReadError`]).

use deino_domain::DisplayName;

use crate::label::Label;

/// Who the meeting window shows as speaking.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Active {
    /// This participant's indicator is lit.
    Speaking(DisplayName),
    /// The user's own tile is lit: not a voice on the system track.
    Me,
    /// Indicators are shown and none is lit.
    Nobody,
    /// More than one indicator is lit (crosstalk): no single name.
    Several,
    /// The window shows participants but no indicator at all (a layout
    /// without one, such as a shared screen with the tiles hidden).
    NoIndicator,
}

impl Active {
    /// The remote participant speaking, if exactly one is.
    pub fn remote(&self) -> Option<&DisplayName> {
        match self {
            Self::Speaking(name) => Some(name),
            Self::Me | Self::Nobody | Self::Several | Self::NoIndicator => None,
        }
    }
}

/// One look at a meeting window.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reading {
    /// Everyone the window names, without the user, in order of appearance.
    pub participants: Vec<DisplayName>,
    /// Who is speaking.
    pub active: Active,
}

/// Why a window gave no reading. Neither is a failure of the recording:
/// the watcher logs it at debug level and the meeting falls back to voice
/// matching for that stretch.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ReadError {
    /// The window is the app's, but no call is under way in it (a lobby,
    /// the chat, the home screen).
    #[error("no call is under way in this window")]
    NotInCall,
    /// A call is under way but the window has none of the elements the
    /// adapter knows: the app's UI changed, or this version is unknown.
    #[error("the call window's layout was not recognised")]
    UnrecognisedLayout,
}

/// What an adapter's scan of a call window found.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Scan {
    /// Every participant label, in order.
    pub(crate) labels: Vec<Label>,
    /// The labels whose indicator is lit.
    pub(crate) lit: Vec<Label>,
    /// How many indicators (lit or not) the window shows.
    pub(crate) indicators: usize,
}

impl Scan {
    /// The scan as a reading, or [`ReadError::UnrecognisedLayout`] when it
    /// found neither participants nor indicators.
    pub(crate) fn into_reading(self) -> Result<Reading, ReadError> {
        if self.labels.is_empty() && self.indicators == 0 {
            return Err(ReadError::UnrecognisedLayout);
        }
        let mut participants: Vec<DisplayName> = Vec::new();
        for label in self.labels.iter().filter(|label| !label.is_self) {
            if !participants.contains(&label.name) {
                participants.push(label.name.clone());
            }
        }
        let active = active(&self.lit, self.indicators, &participants);
        Ok(Reading {
            participants,
            active,
        })
    }
}

/// Who is speaking, from the lit labels.
fn active(lit: &[Label], indicators: usize, participants: &[DisplayName]) -> Active {
    let mut speaking: Vec<&Label> = Vec::new();
    for label in lit {
        if !speaking.iter().any(|known| known.name == label.name) {
            speaking.push(label);
        }
    }
    match speaking.as_slice() {
        [] if indicators == 0 => Active::NoIndicator,
        [] => Active::Nobody,
        [only] if only.is_self => Active::Me,
        [only] => Active::Speaking(expand(&only.name, participants)),
        _ => Active::Several,
    }
}

/// A tile's shortened name ("Alex C.", "Alex Ch…") as the one participant
/// it abbreviates ("Alex Chen"); any other name as it is.
fn expand(name: &DisplayName, participants: &[DisplayName]) -> DisplayName {
    let text = name.as_str();
    let Some(prefix) = text.strip_suffix('…').or_else(|| text.strip_suffix('.')) else {
        return name.clone();
    };
    let prefix = prefix.to_lowercase();
    let mut matches = participants
        .iter()
        .filter(|full| full.key().starts_with(&prefix) && *full != name);
    match (matches.next(), matches.next()) {
        (Some(full), None) => full.clone(),
        _ => name.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn label(raw: &str) -> Label {
        Label::parse(raw).unwrap()
    }

    fn scan(labels: &[&str], lit: &[&str], indicators: usize) -> Result<Reading, ReadError> {
        Scan {
            labels: labels.iter().map(|raw| label(raw)).collect(),
            lit: lit.iter().map(|raw| label(raw)).collect(),
            indicators,
        }
        .into_reading()
    }

    fn name(text: &str) -> DisplayName {
        DisplayName::parse(text).unwrap()
    }

    #[test]
    fn one_lit_remote_tile_is_the_speaker_and_the_user_is_left_out() {
        let reading = scan(&["Alex Chen", "Me (me)", "Bea"], &["Bea"], 3).unwrap();

        assert_eq!(reading.participants, [name("Alex Chen"), name("Bea")]);
        assert_eq!(reading.active, Active::Speaking(name("Bea")));
        assert_eq!(reading.active.remote(), Some(&name("Bea")));
    }

    #[test]
    fn the_users_tile_crosstalk_silence_and_a_missing_indicator_name_nobody() {
        let labels = ["Alex", "Shreyas (You)"];

        assert_eq!(
            scan(&labels, &["Shreyas (You)"], 2).unwrap().active,
            Active::Me
        );
        assert_eq!(
            scan(&labels, &["Alex", "Shreyas (You)"], 2).unwrap().active,
            Active::Several
        );
        assert_eq!(scan(&labels, &[], 2).unwrap().active, Active::Nobody);
        assert_eq!(scan(&labels, &[], 0).unwrap().active, Active::NoIndicator);
        assert!(scan(&labels, &[], 0).unwrap().active.remote().is_none());
    }

    #[test]
    fn a_shortened_tile_name_takes_the_one_full_name_it_abbreviates() {
        let reading = scan(&["Alex Chen", "Alex C."], &["Alex C."], 2).unwrap();
        assert_eq!(reading.active, Active::Speaking(name("Alex Chen")));

        let ambiguous = scan(&["Alex Chen", "Alex Cruz", "Alex C."], &["Alex C."], 3).unwrap();
        assert_eq!(ambiguous.active, Active::Speaking(name("Alex C.")));
    }

    #[test]
    fn a_window_with_nothing_known_is_an_unrecognised_layout() {
        assert_eq!(scan(&[], &[], 0), Err(ReadError::UnrecognisedLayout));
        assert!(scan(&[], &[], 1).is_ok());
    }
}
