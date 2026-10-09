//! Centroid-linkage agglomerative clustering (the dendrogram) and cutting it.
//!
//! Adapted from anarlog `crates/pyannote-local/src/clustering.rs` (MIT).
//! Deino's change: each cluster's centroid is kept up to date on merge
//! instead of being recomputed from its sum for every distance, which gives
//! the same values without an allocation per pair. Nearest-neighbour chains
//! keep the run at about O(n²·d) for n anchors.

use crate::vector::{euclidean, normalize};

/// One merge of the dendrogram: cluster `right` joined cluster `left`
/// (identified by their lowest original item) at centroid distance
/// `distance`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Merge {
    /// The surviving cluster.
    pub left: usize,
    /// The absorbed cluster.
    pub right: usize,
    /// Euclidean distance between the two centroids when they merged.
    pub distance: f32,
}

/// The `n - 1` merges of `embeddings` (normalised first), in merge order.
pub fn linkage(embeddings: &[Vec<f32>]) -> Vec<Merge> {
    let mut state = State::new(embeddings);
    let mut merges = Vec::with_capacity(embeddings.len().saturating_sub(1));
    while let Some(merge) = state.closest_pair() {
        state.merge(merge.left, merge.right);
        merges.push(merge);
    }
    merges
}

/// Labels after the first `merge_count` merges, compacted to `0..k` in
/// order of first appearance.
pub fn cut(item_count: usize, merges: &[Merge], merge_count: usize) -> Vec<usize> {
    let mut sets = DisjointSets::new(item_count);
    for merge in merges.iter().take(merge_count) {
        sets.union(merge.left, merge.right);
    }
    let mut lookup: Vec<Option<usize>> = vec![None; item_count];
    let mut next = 0;
    (0..item_count)
        .map(|item| {
            let root = sets.find(item);
            match lookup.get_mut(root) {
                Some(Some(label)) => *label,
                Some(slot) => {
                    *slot = Some(next);
                    next += 1;
                    next - 1
                }
                None => 0,
            }
        })
        .collect()
}

/// Union-find with path halving and union by size.
pub(super) struct DisjointSets {
    parent: Vec<usize>,
    size: Vec<usize>,
}

impl DisjointSets {
    pub(super) fn new(count: usize) -> Self {
        Self {
            parent: (0..count).collect(),
            size: vec![1; count],
        }
    }

    pub(super) fn find(&mut self, mut item: usize) -> usize {
        while let Some(&parent) = self.parent.get(item) {
            if parent == item {
                break;
            }
            let grandparent = self.parent.get(parent).copied().unwrap_or(parent);
            if let Some(slot) = self.parent.get_mut(item) {
                *slot = grandparent;
            }
            item = grandparent;
        }
        item
    }

    /// Joins the sets of `left` and `right`; the sizes of the two sets
    /// before the join, or `None` when they were already one.
    pub(super) fn union(&mut self, left: usize, right: usize) -> Option<(usize, usize)> {
        let (left, right) = (self.find(left), self.find(right));
        if left == right {
            return None;
        }
        let sizes = (self.size_of(left), self.size_of(right));
        let (root, child) = if sizes.0 >= sizes.1 {
            (left, right)
        } else {
            (right, left)
        };
        if let Some(slot) = self.parent.get_mut(child) {
            *slot = root;
        }
        if let Some(slot) = self.size.get_mut(root) {
            *slot = sizes.0 + sizes.1;
        }
        Some(sizes)
    }

    fn size_of(&self, root: usize) -> usize {
        self.size.get(root).copied().unwrap_or(1)
    }
}

/// The working state of [`linkage`]: live clusters, their centroids, the
/// pairwise distance matrix and each row's nearest neighbour.
struct State {
    count: usize,
    centroids: Vec<Vec<f32>>,
    sums: Vec<Vec<f32>>,
    sizes: Vec<usize>,
    alive: Vec<bool>,
    distances: Vec<f32>,
    nearest: Vec<(usize, f32)>,
}

impl State {
    fn new(embeddings: &[Vec<f32>]) -> Self {
        let count = embeddings.len();
        let unit: Vec<Vec<f32>> = embeddings.iter().map(|e| normalize(e)).collect();
        let mut distances = vec![f32::INFINITY; count * count];
        for (i, left) in unit.iter().enumerate() {
            for (j, right) in unit.iter().enumerate().skip(i + 1) {
                let distance = euclidean(left, right);
                set(&mut distances, i * count + j, distance);
                set(&mut distances, j * count + i, distance);
            }
        }
        let mut state = Self {
            count,
            sums: unit.clone(),
            centroids: unit,
            sizes: vec![1; count],
            alive: vec![true; count],
            distances,
            nearest: Vec::new(),
        };
        state.nearest = (0..count).map(|row| state.nearest_of(row)).collect();
        state
    }

    fn distance(&self, row: usize, col: usize) -> f32 {
        self.distances
            .get(row * self.count + col)
            .copied()
            .unwrap_or(f32::INFINITY)
    }

    fn is_alive(&self, row: usize) -> bool {
        self.alive.get(row).copied().unwrap_or(false)
    }

    /// The closest live cluster to `row`, `(usize::MAX, ∞)` when none.
    fn nearest_of(&self, row: usize) -> (usize, f32) {
        (0..self.count)
            .filter(|col| *col != row && self.is_alive(*col))
            .map(|col| (col, self.distance(row, col)))
            .fold((usize::MAX, f32::INFINITY), |best, candidate| {
                if candidate.1 < best.1 {
                    candidate
                } else {
                    best
                }
            })
    }

    /// The live pair with the smallest distance, as a merge.
    fn closest_pair(&self) -> Option<Merge> {
        let (row, (col, distance)) = self
            .nearest
            .iter()
            .enumerate()
            .filter(|(row, (col, _))| self.is_alive(*row) && *col != usize::MAX)
            .fold(
                None,
                |best: Option<(usize, (usize, f32))>, (row, near)| match best {
                    Some(best) if best.1.1 <= near.1 => Some(best),
                    _ => Some((row, *near)),
                },
            )?;
        Some(Merge {
            left: row.min(col),
            right: row.max(col),
            distance,
        })
    }

    /// Folds cluster `drop` into `keep` and updates distances and neighbours.
    fn merge(&mut self, keep: usize, drop: usize) {
        let dropped = self
            .sums
            .get_mut(drop)
            .map(std::mem::take)
            .unwrap_or_default();
        let dropped_size = self.sizes.get(drop).copied().unwrap_or(0);
        if let Some(alive) = self.alive.get_mut(drop) {
            *alive = false;
        }
        let size = self.sizes.get(keep).copied().unwrap_or(0) + dropped_size;
        if let (Some(sum), Some(slot)) = (self.sums.get_mut(keep), self.sizes.get_mut(keep)) {
            sum.iter_mut()
                .zip(&dropped)
                .for_each(|(total, value)| *total += value);
            *slot = size;
        }
        self.refresh_centroid(keep);
        self.refresh_distances(keep);
        self.refresh_neighbours(keep, drop);
    }

    fn refresh_centroid(&mut self, keep: usize) {
        #[allow(clippy::cast_precision_loss, reason = "cluster sizes stay below 2^24")]
        let size = self.sizes.get(keep).copied().unwrap_or(1).max(1) as f32;
        let centroid: Vec<f32> = self
            .sums
            .get(keep)
            .map(|sum| sum.iter().map(|value| value / size).collect())
            .unwrap_or_default();
        if let Some(slot) = self.centroids.get_mut(keep) {
            *slot = centroid;
        }
    }

    fn refresh_distances(&mut self, keep: usize) {
        let empty = Vec::new();
        let centroid = self.centroids.get(keep).unwrap_or(&empty);
        let updates: Vec<(usize, f32)> = (0..self.count)
            .filter(|col| *col != keep && self.is_alive(*col))
            .map(|col| {
                let other = self.centroids.get(col).unwrap_or(&empty);
                (col, euclidean(centroid, other))
            })
            .collect();
        for (col, distance) in updates {
            set(&mut self.distances, keep * self.count + col, distance);
            set(&mut self.distances, col * self.count + keep, distance);
        }
    }

    fn refresh_neighbours(&mut self, keep: usize, drop: usize) {
        for row in 0..self.count {
            if !self.is_alive(row) {
                continue;
            }
            let (near, near_distance) = self
                .nearest
                .get(row)
                .copied()
                .unwrap_or((usize::MAX, f32::INFINITY));
            let to_keep = self.distance(row, keep);
            let next = if row == keep || near == keep || near == drop {
                self.nearest_of(row)
            } else if to_keep < near_distance {
                (keep, to_keep)
            } else {
                continue;
            };
            set_pair(&mut self.nearest, row, next);
        }
    }
}

fn set(values: &mut [f32], index: usize, value: f32) {
    if let Some(slot) = values.get_mut(index) {
        *slot = value;
    }
}

fn set_pair(values: &mut [(usize, f32)], index: usize, value: (usize, f32)) {
    if let Some(slot) = values.get_mut(index) {
        *slot = value;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_closest_pair_merges_first() {
        let embeddings = vec![vec![1.0, 0.0], vec![0.0, 1.0], vec![0.99, 0.1]];
        let merges = linkage(&embeddings);
        assert_eq!(merges.len(), 2);
        assert_eq!((merges[0].left, merges[0].right), (0, 2));
        assert!(merges[0].distance < merges[1].distance);
    }

    #[test]
    fn cut_labels_by_first_appearance() {
        let merges = [
            Merge {
                left: 1,
                right: 3,
                distance: 0.1,
            },
            Merge {
                left: 0,
                right: 2,
                distance: 0.2,
            },
            Merge {
                left: 0,
                right: 1,
                distance: 0.9,
            },
        ];
        assert_eq!(cut(4, &merges, 0), vec![0, 1, 2, 3]);
        assert_eq!(cut(4, &merges, 2), vec![0, 1, 0, 1]);
        assert_eq!(cut(4, &merges, 3), vec![0, 0, 0, 0]);
    }

    #[test]
    fn single_or_no_items_have_no_merges() {
        assert!(linkage(&[]).is_empty());
        assert!(linkage(&[vec![1.0, 0.0]]).is_empty());
    }

    #[test]
    fn union_reports_set_sizes_once() {
        let mut sets = DisjointSets::new(3);
        assert_eq!(sets.union(0, 1), Some((1, 1)));
        assert_eq!(sets.union(1, 2), Some((2, 1)));
        assert_eq!(sets.union(0, 2), None);
        assert_eq!(sets.find(2), sets.find(0));
    }
}
