//! From evidence to at most one [`Candidate`], in Muesli's priority order
//! (`MeetingCandidateResolver.swift`, MIT):
//!
//! 1. a browser meeting room whose browser is capturing the microphone;
//! 2. a meeting room in the focused browser while the microphone runs;
//! 3. a dedicated meeting app capturing (weak apps in full duplex only);
//! 4. the frontmost meeting app while microphone and camera both run;
//! 5. a browser capturing without a known room;
//! 6. any other app capturing.
//!
//! Two deliberate differences from Muesli: a calendar event never creates a
//! candidate on its own, it links to (and strengthens) whatever is found;
//! and weak evidence (5, 6) is ranked last with a sustained dwell, so it can
//! never delay a real meeting app. Denied and ignored apps never count.

use std::collections::HashSet;
use std::hash::BuildHasher;

use deino_domain::BundleId;

use crate::classify::{AppRole, role_of};
use crate::model::{AppIdentity, MicClient, Platform};
use crate::snapshot::{BrowserMeeting, Candidate, CandidateKey, Dwell, SignalSnapshot};

/// The meeting `snapshot` points to, ignoring the apps in `ignored`.
pub fn resolve<S: BuildHasher>(
    snapshot: &SignalSnapshot,
    ignored: &HashSet<BundleId, S>,
) -> Option<Candidate> {
    let evidence = Evidence::new(snapshot, ignored);
    let found = evidence
        .room_in_call()
        .or_else(|| evidence.focused_room())
        .or_else(|| evidence.meeting_app())
        .or_else(|| evidence.foreground_camera_app())
        .or_else(|| evidence.browser_audio())
        .or_else(|| evidence.other_app())?;
    Some(link_event(found, snapshot))
}

/// A calendar event corroborates any candidate and names it.
fn link_event(mut candidate: Candidate, snapshot: &SignalSnapshot) -> Candidate {
    if let Some(event) = &snapshot.event {
        candidate.event = Some(event.clone());
        candidate.dwell = Dwell::Short;
    }
    candidate
}

/// The snapshot with denied and ignored apps removed.
struct Evidence<'a, S> {
    snapshot: &'a SignalSnapshot,
    ignored: &'a HashSet<BundleId, S>,
    clients: Vec<&'a MicClient>,
    rooms: Vec<&'a BrowserMeeting>,
}

impl<'a, S: BuildHasher> Evidence<'a, S> {
    fn new(snapshot: &'a SignalSnapshot, ignored: &'a HashSet<BundleId, S>) -> Self {
        let mut clients: Vec<_> = snapshot
            .clients
            .iter()
            .filter(|client| allowed(&client.app, ignored))
            .collect();
        clients.sort_by(|a, b| a.app.name.cmp(&b.app.name));
        let mut rooms: Vec<_> = snapshot
            .browser_meetings
            .iter()
            .filter(|room| allowed(&room.browser, ignored))
            .collect();
        // Focused first, then Meet (Muesli's tie-break), then by browser name.
        rooms.sort_by_key(|room| {
            let not_meet = room.url.platform != Platform::GoogleMeet;
            (!room.focused, not_meet, room.browser.name.clone())
        });
        Self {
            snapshot,
            ignored,
            clients,
            rooms,
        }
    }

    fn capturing(&self) -> bool {
        self.snapshot.microphone_running || !self.clients.is_empty()
    }

    fn room_in_call(&self) -> Option<Candidate> {
        let room = self
            .rooms
            .iter()
            .find(|room| self.client_of(&room.browser.bundle_id).is_some())?;
        Some(room_candidate(room))
    }

    fn focused_room(&self) -> Option<Candidate> {
        let room = self.rooms.iter().find(|room| room.focused)?;
        self.capturing().then(|| room_candidate(room))
    }

    fn meeting_app(&self) -> Option<Candidate> {
        // Strong apps before weak ones; names break ties.
        let mut apps: Vec<_> = self
            .clients
            .iter()
            .filter_map(|client| match role_of(&client.app.bundle_id) {
                AppRole::Meeting { platform, weak } if !weak || client.full_duplex => {
                    Some((weak, platform, *client))
                }
                _ => None,
            })
            .collect();
        apps.sort_by_key(|(weak, _, _)| *weak);
        let (_, platform, client) = apps.first()?;
        Some(app_candidate(&client.app, *platform, Dwell::Short))
    }

    fn foreground_camera_app(&self) -> Option<Candidate> {
        if !(self.snapshot.microphone_running && self.snapshot.camera_running) {
            return None;
        }
        let front = self.snapshot.frontmost.as_ref()?;
        match role_of(&front.bundle_id) {
            AppRole::Meeting { platform, .. } if allowed(front, self.ignored) => {
                Some(app_candidate(front, platform, Dwell::Short))
            }
            _ => None,
        }
    }

    fn browser_audio(&self) -> Option<Candidate> {
        let client = self
            .clients
            .iter()
            .find(|client| role_of(&client.app.bundle_id) == AppRole::Browser)?;
        Some(app_candidate(
            &client.app,
            Platform::Other,
            Dwell::Sustained,
        ))
    }

    fn other_app(&self) -> Option<Candidate> {
        let client = self
            .clients
            .iter()
            .find(|client| role_of(&client.app.bundle_id) == AppRole::Unknown)?;
        Some(app_candidate(
            &client.app,
            Platform::Other,
            Dwell::Sustained,
        ))
    }

    fn client_of(&self, bundle_id: &BundleId) -> Option<&MicClient> {
        self.clients
            .iter()
            .copied()
            .find(|client| &client.app.bundle_id == bundle_id)
    }
}

/// `app` may be a meeting: it is neither denied nor on the ignore list.
fn allowed<S: BuildHasher>(app: &AppIdentity, ignored: &HashSet<BundleId, S>) -> bool {
    !ignored.contains(&app.bundle_id) && role_of(&app.bundle_id) != AppRole::Denied
}

fn room_candidate(room: &BrowserMeeting) -> Candidate {
    Candidate {
        key: CandidateKey::room(&room.url),
        app: room.browser.clone(),
        platform: room.url.platform,
        dwell: Dwell::Short,
        event: None,
    }
}

fn app_candidate(app: &AppIdentity, platform: Platform, dwell: Dwell) -> Candidate {
    Candidate {
        key: CandidateKey::app(&app.bundle_id),
        app: app.clone(),
        platform,
        dwell,
        event: None,
    }
}

#[cfg(test)]
#[path = "resolver_tests.rs"]
mod tests;
