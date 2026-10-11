//! Where the Mac's sound goes: speakers (it can reach the microphone) or
//! headphones (it cannot).
//!
//! The echo canceller is only needed when meeting audio can leak from the
//! speakers into the microphone. Following anarlog's
//! `headphone_only_output`, the route is "headphones" when every output
//! device other apps are playing to is a headphone, or, when nothing plays
//! yet, when the default output is one. [`RouteTracker`] only reports a
//! change after two consecutive probes agree, so a one-off system sound
//! cannot flip the verdict (anarlog's `OutputRoutingTracker`).

use std::time::Duration;

/// How often the supervisor probes the route while recording.
pub const ROUTE_POLL: Duration = Duration::from_secs(2);

/// Where audio output goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OutputRoute {
    /// Speakers (or anything that is not a headphone): audio can reach the mic.
    Speakers,
    /// Every playing output is a headphone: the microphone hears no playback.
    Headphones,
}

/// Reads the current output route.
pub trait RouteProbe: std::fmt::Debug + Send + Sync {
    /// The route now, or `None` when it cannot be read.
    fn route(&self) -> Option<OutputRoute>;
}

/// A probe that always answers the same (tests, mock audio, other platforms).
#[derive(Clone, Copy, Debug)]
pub struct FixedRoute(pub Option<OutputRoute>);

impl RouteProbe for FixedRoute {
    fn route(&self) -> Option<OutputRoute> {
        self.0
    }
}

/// An output device as the headphone verdict sees it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutputDevice {
    /// Its name.
    pub name: String,
    /// Whether Core Audio (or its name) says it is a headphone.
    pub is_headphone: bool,
}

/// The route given the default output and the devices other apps play to.
pub fn verdict(default: Option<&OutputDevice>, playing: &[OutputDevice]) -> Option<OutputRoute> {
    let headphones = if playing.is_empty() {
        default?.is_headphone
    } else {
        playing.iter().all(|device| device.is_headphone)
    };
    Some(if headphones {
        OutputRoute::Headphones
    } else {
        OutputRoute::Speakers
    })
}

/// Name hints of headphones whose streams do not say so (anarlog).
const HEADPHONE_HINTS: [&str; 5] = ["headphone", "headset", "airpod", "earbud", "buds"];

/// Name hints of Bluetooth speakers, which are not headphones.
const SPEAKER_HINTS: [&str; 3] = ["speaker", "soundbar", "homepod"];

/// Whether a device whose streams do not report a terminal type is a
/// headphone, judged by transport and name.
pub fn headphone_by_name(name: &str, bluetooth: bool) -> bool {
    let lower = name.to_lowercase();
    if bluetooth {
        return !SPEAKER_HINTS.iter().any(|hint| lower.contains(hint));
    }
    HEADPHONE_HINTS.iter().any(|hint| lower.contains(hint))
}

/// Debounces route probes: a new route is accepted after two equal probes.
#[derive(Clone, Copy, Debug)]
pub struct RouteTracker {
    current: OutputRoute,
    candidate: Option<OutputRoute>,
}

impl RouteTracker {
    /// A tracker that starts at `initial` (accepted immediately).
    pub fn new(initial: OutputRoute) -> Self {
        Self {
            current: initial,
            candidate: None,
        }
    }

    /// The accepted route.
    pub fn current(&self) -> OutputRoute {
        self.current
    }

    /// Feeds one probe; returns the new route when it changes.
    pub fn observe(&mut self, probe: Option<OutputRoute>) -> Option<OutputRoute> {
        let route = probe?;
        if route == self.current {
            self.candidate = None;
            return None;
        }
        if self.candidate == Some(route) {
            self.current = route;
            self.candidate = None;
            return Some(route);
        }
        self.candidate = Some(route);
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn output(name: &str, is_headphone: bool) -> OutputDevice {
        OutputDevice {
            name: name.into(),
            is_headphone,
        }
    }

    #[test]
    fn with_nothing_playing_the_default_output_decides() {
        let airpods = output("AirPods Pro", true);
        assert_eq!(verdict(Some(&airpods), &[]), Some(OutputRoute::Headphones));
        let speakers = output("MacBook Pro Speakers", false);
        assert_eq!(verdict(Some(&speakers), &[]), Some(OutputRoute::Speakers));
        assert_eq!(verdict(None, &[]), None);
    }

    #[test]
    fn every_playing_output_must_be_a_headphone() {
        let speakers = output("MacBook Pro Speakers", false);
        let playing = [output("AirPods Pro", true), output("Display Audio", false)];
        assert_eq!(
            verdict(Some(&speakers), &playing[..1]),
            Some(OutputRoute::Headphones)
        );
        assert_eq!(
            verdict(Some(&speakers), &playing),
            Some(OutputRoute::Speakers)
        );
    }

    #[test]
    fn names_decide_for_devices_without_a_headphone_terminal() {
        assert!(headphone_by_name("Bose QC Headphones", false));
        assert!(!headphone_by_name("LG Monitor", false));
        assert!(headphone_by_name("Jabra Elite", true));
        assert!(!headphone_by_name("JBL Flip Speaker", true));
    }

    #[test]
    fn a_route_change_needs_two_agreeing_probes() {
        let mut tracker = RouteTracker::new(OutputRoute::Speakers);
        assert_eq!(tracker.observe(Some(OutputRoute::Headphones)), None);
        assert_eq!(tracker.observe(Some(OutputRoute::Speakers)), None);
        assert_eq!(tracker.observe(Some(OutputRoute::Headphones)), None);
        assert_eq!(
            tracker.observe(Some(OutputRoute::Headphones)),
            Some(OutputRoute::Headphones)
        );
        assert_eq!(tracker.current(), OutputRoute::Headphones);
        assert_eq!(tracker.observe(None), None);
    }
}
