//! Reciprocal rank fusion of keyword and semantic hits (Cormack, Clarke and
//! Büttcher, SIGIR 2009).
//!
//! Each list ranks its hits; a hit's fused score is the sum over the lists
//! that found it of `1 / (RRF_K + rank)`. RRF needs no score calibration
//! between BM25 and cosine similarity, which live on unrelated scales.
//! Ranks count within a kind, because both lists are cut per kind and the
//! results are shown per kind.
//!
//! The lists find the same thing under different keys: a keyword moment is
//! one transcript segment, a semantic moment a window of segments. The best
//! keyword moment inside a window takes the window's key, so the two fuse;
//! the fused hit keeps the keyword hit's precise time and highlights.

use std::collections::HashMap;

use deino_domain::MeetingId;

use super::{Hit, HitKind};

/// Rank damping: hits past the first few dozen weigh about the same.
pub const RRF_K: f64 = 60.0;

/// What makes two hits the same result.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum Key {
    /// A document: its kind and ID.
    Ref(HitKind, String),
    /// A transcript window: its meeting and start.
    Window(MeetingId, u32),
}

/// A result being fused.
struct Fused {
    hit: Hit,
    score: f64,
    from_keyword: bool,
}

/// Fuses two ranked lists (best first) into one, best first.
pub fn fuse(keyword: Vec<Hit>, semantic: Vec<Hit>) -> Vec<Hit> {
    let semantic = with_keys(semantic, |hit| {
        window_key(hit).unwrap_or_else(|| ref_key(hit))
    });
    let windows: Vec<&Hit> = semantic.iter().map(|(_, hit)| hit).collect();
    let keyword = claim_windows(keyword, &windows);
    let mut fused: Vec<Fused> = Vec::new();
    let mut slots: HashMap<Key, usize> = HashMap::new();
    for (list, from_keyword) in [(keyword, true), (semantic, false)] {
        for (rank, (key, hit)) in ranked_within_kind(list) {
            let score = 1.0 / (RRF_K + f64::from(rank));
            add(&mut fused, &mut slots, key, (hit, score, from_keyword));
        }
    }
    let mut hits: Vec<Hit> = fused
        .into_iter()
        .map(|entry| Hit {
            score: entry.score,
            ..entry.hit
        })
        .collect();
    hits.sort_by(|a, b| b.score.total_cmp(&a.score));
    hits
}

/// Adds one list's hit to the fusion, or its score to the hit already there.
fn add(
    fused: &mut Vec<Fused>,
    slots: &mut HashMap<Key, usize>,
    key: Key,
    (hit, score, from_keyword): (Hit, f64, bool),
) {
    let Some(&slot) = slots.get(&key) else {
        slots.insert(key, fused.len());
        fused.push(Fused {
            hit,
            score,
            from_keyword,
        });
        return;
    };
    if let Some(entry) = fused.get_mut(slot) {
        entry.score += score;
        // Keyword hits carry highlights and exact times: keep them.
        if from_keyword && !entry.from_keyword {
            entry.hit = hit;
            entry.from_keyword = true;
        }
    }
}

/// Keyword hits with their keys: the best moment inside each semantic
/// window takes that window's key.
fn claim_windows(keyword: Vec<Hit>, windows: &[&Hit]) -> Vec<(Key, Hit)> {
    let mut claimed: Vec<Key> = Vec::new();
    with_keys(keyword, |hit| {
        let window = windows
            .iter()
            .filter_map(|window| covering(window, hit))
            .find(|key| !claimed.contains(key));
        match window {
            Some(key) => {
                claimed.push(key.clone());
                key
            }
            None => ref_key(hit),
        }
    })
}

/// The window's key when `window` is a transcript window of `hit`'s meeting
/// that spans `hit`'s time.
fn covering(window: &Hit, hit: &Hit) -> Option<Key> {
    let key = window_key(window)?;
    let (start, end, at) = (window.ts_ms?, window.end_ms?, hit.ts_ms?);
    let same_meeting = hit.kind == HitKind::Segment && window.meeting_id == hit.meeting_id;
    (same_meeting && start <= at && at <= end).then_some(key)
}

fn window_key(hit: &Hit) -> Option<Key> {
    match (hit.kind, &hit.meeting_id, hit.ts_ms, hit.end_ms) {
        (HitKind::Segment, Some(meeting), Some(start), Some(_)) => {
            Some(Key::Window(meeting.clone(), start.get()))
        }
        _ => None,
    }
}

fn ref_key(hit: &Hit) -> Key {
    Key::Ref(hit.kind, hit.ref_id.clone())
}

fn with_keys(hits: Vec<Hit>, mut key: impl FnMut(&Hit) -> Key) -> Vec<(Key, Hit)> {
    hits.into_iter().map(|hit| (key(&hit), hit)).collect()
}

/// Each hit with its 1-based rank among the list's hits of its kind.
fn ranked_within_kind(list: Vec<(Key, Hit)>) -> Vec<(u32, (Key, Hit))> {
    let mut counts: HashMap<HitKind, u32> = HashMap::new();
    list.into_iter()
        .map(|(key, hit)| {
            let rank = counts.entry(hit.kind).or_insert(0);
            *rank += 1;
            (*rank, (key, hit))
        })
        .collect()
}

#[cfg(test)]
#[path = "hybrid_tests.rs"]
mod tests;
