//! Stage 3: known voiceprints steer the clusters.
//!
//! Fragmenting one voice into several clusters is local diarization's most
//! common failure. Clusters whose best known match is the same person merge
//! (never below the speaker-count floor), and each surviving cluster is
//! named when the match is unique in both directions. Ported from anarlog
//! `pipeline.rs::apply_known_speakers` (MIT).

use deino_domain::PersonId;

use super::labels::Clusters;
use super::scan::WindowEmbedding;
use super::{DiarizeRequest, KnownSpeaker, SpeakerIdentity};
use crate::clustering;
use crate::config::DiarizationConfig;
use crate::vector::cosine_similarity;
use deino_core::voiceprints::matching::{
    MIN_UNIQUE_MARGIN, MIN_UNIQUE_SCORE, MatchRule, pick_unique,
};

/// Merges clusters of the same known voice, then names each cluster that
/// matches a known person uniquely (`clusters.identities`).
pub(super) fn apply(
    embeddings: &[WindowEmbedding],
    clusters: &mut Clusters,
    request: &DiarizeRequest<'_>,
    config: &DiarizationConfig,
) {
    let known = request.known_speakers;
    if known.is_empty() || clusters.centroids.is_empty() {
        return;
    }
    let people = distinct_people(known);
    let floor = request.bounds.floor();
    let best: Vec<Option<PersonId>> = clusters
        .centroids
        .iter()
        .map(|centroid| best_person(centroid, &people, known, config.known_speaker_min_score))
        .collect();
    if merge_same_person(clusters, &best, &people, floor) {
        let vectors: Vec<Vec<f32>> = embeddings
            .iter()
            .map(|item| item.embedding.clone())
            .collect();
        clusters.centroids = clustering::centroids(&vectors, &clusters.labels);
    }
    clusters.identities = identify(&clusters.centroids, &people, known, config);
}

/// Every person once, in first-seen order.
fn distinct_people(known: &[KnownSpeaker]) -> Vec<PersonId> {
    let mut people: Vec<PersonId> = Vec::new();
    for speaker in known {
        if !people.contains(&speaker.person) {
            people.push(speaker.person);
        }
    }
    people
}

/// `person`'s score for `centroid`: the best over their exemplars.
fn score(centroid: &[f32], person: PersonId, known: &[KnownSpeaker]) -> Option<f32> {
    known
        .iter()
        .filter(|speaker| speaker.person == person)
        .filter_map(|speaker| cosine_similarity(centroid, speaker.embedding.as_slice()))
        .max_by(f32::total_cmp)
}

/// The best-scoring person for `centroid`, if they reach `min_score`.
fn best_person(
    centroid: &[f32],
    people: &[PersonId],
    known: &[KnownSpeaker],
    min_score: f32,
) -> Option<PersonId> {
    people
        .iter()
        .filter_map(|person| score(centroid, *person, known).map(|score| (*person, score)))
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .filter(|(_, score)| *score >= min_score)
        .map(|(person, _)| person)
}

/// Folds, person by person, every further cluster that matched them into
/// their first one while more than `floor` clusters remain, then renumbers
/// the labels by first appearance. Whether anything merged.
fn merge_same_person(
    clusters: &mut Clusters,
    best: &[Option<PersonId>],
    people: &[PersonId],
    floor: usize,
) -> bool {
    let mut target: Vec<usize> = (0..best.len()).collect();
    let mut remaining = best.len();
    for person in people {
        let matching: Vec<usize> = (0..best.len())
            .filter(|cluster| best.get(*cluster).copied().flatten() == Some(*person))
            .collect();
        let Some((&first, rest)) = matching.split_first() else {
            continue;
        };
        for cluster in rest {
            if remaining <= floor {
                break;
            }
            if let Some(slot) = target.get_mut(*cluster) {
                *slot = first;
            }
            remaining -= 1;
        }
    }
    if remaining == best.len() {
        return false;
    }
    compact(&mut clusters.labels, &target);
    true
}

/// Maps each label through `target` and renumbers by first appearance.
fn compact(labels: &mut [usize], target: &[usize]) {
    let mut renumbered: Vec<Option<usize>> = vec![None; target.len()];
    let mut next = 0;
    for label in labels {
        let merged = target.get(*label).copied().unwrap_or(*label);
        *label = match renumbered.get_mut(merged) {
            Some(Some(number)) => *number,
            Some(empty) => {
                *empty = Some(next);
                next += 1;
                next - 1
            }
            None => merged,
        };
    }
}

/// Names each cluster that is a known person's unique, mutual best match.
fn identify(
    centroids: &[Vec<f32>],
    people: &[PersonId],
    known: &[KnownSpeaker],
    config: &DiarizationConfig,
) -> Vec<Option<SpeakerIdentity>> {
    let scores: Vec<(usize, PersonId, f32)> = centroids
        .iter()
        .enumerate()
        .flat_map(|(cluster, centroid)| {
            people.iter().filter_map(move |person| {
                score(centroid, *person, known).map(|s| (cluster, *person, s))
            })
        })
        .collect();
    let rule = MatchRule {
        min_score: config.known_speaker_min_score.max(MIN_UNIQUE_SCORE),
        min_margin: MIN_UNIQUE_MARGIN,
    };
    let mut identities = vec![None; centroids.len()];
    for assignment in pick_unique(&scores, rule) {
        if let Some(slot) = identities.get_mut(assignment.speaker) {
            *slot = Some(SpeakerIdentity {
                person: assignment.person,
                score: assignment.score,
            });
        }
    }
    identities
}
