//! [`Presence`]: who the call window listed, poll by poll, as spans: the
//! remote roster over the recording. Each participant has their own
//! [`Timeline`], so a roster span starts and ends halfway between polls and
//! closes when polls stop, exactly like a speaker span.
//!
//! `identify_speakers` reads the roster to tell a one-to-one call (one
//! remote person throughout) from a group call (`deino-core`'s
//! `speaker_tags::one_to_one`).

use deino_domain::{CallApp, DisplayName, Millis, TagSpan};

use crate::timeline::{Sighting, Timeline};

/// Every remote participant seen so far, with the spans they were listed.
#[derive(Clone, Debug, Default)]
pub struct Presence {
    lines: Vec<(Sighting, Timeline)>,
}

impl Presence {
    /// Records one poll at `at`: `app`'s call window listed `names`, or no
    /// call window was read (`None`).
    pub fn observe(&mut self, at: Millis, listed: Option<(CallApp, &[DisplayName])>) {
        let listed: Vec<Sighting> = listed
            .map(|(app, names)| names.iter().map(|name| (app, name.clone())).collect())
            .unwrap_or_default();
        for (who, line) in &mut self.lines {
            let here = listed.contains(who);
            line.observe(at, here.then(|| who.clone()));
        }
        for who in listed {
            if !self.lines.iter().any(|(known, _)| *known == who) {
                let mut line = Timeline::default();
                line.observe(at, Some(who.clone()));
                self.lines.push((who, line));
            }
        }
    }

    /// Polls stop for now (a pause): every open span ends.
    pub fn interrupt(&mut self) {
        for (_, line) in &mut self.lines {
            line.interrupt();
        }
    }

    /// The roster spans closed since the last call, in time order.
    pub fn take_closed(&mut self) -> Vec<TagSpan> {
        let mut spans: Vec<TagSpan> = self
            .lines
            .iter_mut()
            .flat_map(|(_, line)| line.take_closed())
            .collect();
        spans.sort_by_key(|span| span.start);
        spans
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(list: &[&str]) -> Vec<DisplayName> {
        list.iter()
            .map(|name| DisplayName::parse(name).unwrap())
            .collect()
    }

    #[test]
    fn each_listed_participant_gets_spans_of_their_own() {
        let mut presence = Presence::default();
        let both = names(&["Bea Ruiz", "Cleo Martin"]);
        let bea = names(&["Bea Ruiz"]);

        presence.observe(Millis(0), Some((CallApp::Zoom, &both)));
        presence.observe(Millis(250), Some((CallApp::Zoom, &both)));
        presence.observe(Millis(500), Some((CallApp::Zoom, &bea)));
        presence.observe(Millis(750), None);
        presence.interrupt();

        let spans: Vec<(u32, u32, String)> = presence
            .take_closed()
            .into_iter()
            .map(|span| (span.start.0, span.end.0, span.name.to_string()))
            .collect();
        assert_eq!(
            spans,
            [(0, 625, "Bea Ruiz".into()), (0, 375, "Cleo Martin".into())]
        );
    }
}
