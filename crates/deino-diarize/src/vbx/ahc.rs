//! The first pass, as Community-1 runs it: centroid-linkage agglomerative
//! clustering of the unit embeddings by euclidean distance (`scipy`'s
//! `linkage(method="centroid")`), cut where a merge's height, or any height
//! below it, passes the threshold (`fcluster(criterion="distance")`).
//!
//! Merges keep the squared distances between centroids up to date with the
//! Lance–Williams rule, and each cluster remembers its nearest neighbour, so
//! a merge costs one pass over the clusters (scipy's "generic" algorithm).

/// Each vector's flat cluster, numbered by first appearance.
pub fn clusters(unit: &[Vec<f64>], threshold: f64) -> Vec<usize> {
    let count = unit.len();
    let mut table = Table::new(unit);
    let mut size = vec![1.0_f64; count];
    let mut height = vec![0.0_f64; count];
    let mut active: Vec<bool> = vec![true; count];
    let mut flat: Vec<usize> = (0..count).collect();
    let mut nearest: Vec<(f64, usize)> = (0..count).map(|i| table.nearest(i, &active)).collect();
    for _ in 1..count {
        let Some((a, b, d2)) = closest_pair(&nearest, &active) else {
            break;
        };
        let value = |v: &[f64], i: usize| v.get(i).copied().unwrap_or_default();
        let merged = value(&height, a)
            .max(value(&height, b))
            .max(d2.max(0.0).sqrt());
        if merged <= threshold {
            relabel(&mut flat, b, a);
        }
        let sizes = (value(&size, a), value(&size, b));
        table.merge((a, b), sizes, &active);
        if let Some(slot) = size.get_mut(a) {
            *slot = sizes.0 + sizes.1;
        }
        if let Some(slot) = height.get_mut(a) {
            *slot = merged;
        }
        if let Some(slot) = active.get_mut(b) {
            *slot = false;
        }
        refresh(&mut nearest, &table, &active, (a, b));
    }
    renumber(&flat)
}

/// The active pair with the smallest distance.
fn closest_pair(nearest: &[(f64, usize)], active: &[bool]) -> Option<(usize, usize, f64)> {
    nearest
        .iter()
        .enumerate()
        .filter(|(i, (_, j))| active.get(*i) == Some(&true) && *i != *j)
        .min_by(|x, y| x.1.0.total_cmp(&y.1.0))
        .map(|(i, &(d2, j))| (i.min(j), i.max(j), d2))
}

/// Updates the nearest neighbours after `b` merged into `a`.
fn refresh(nearest: &mut [(f64, usize)], table: &Table, active: &[bool], (a, b): (usize, usize)) {
    for i in 0..nearest.len() {
        if active.get(i) != Some(&true) || i == a {
            continue;
        }
        let Some(entry) = nearest.get_mut(i) else {
            continue;
        };
        let to_a = table.get(i, a);
        if entry.1 == a || entry.1 == b {
            *entry = table.nearest(i, active);
        } else if to_a < entry.0 {
            *entry = (to_a, a);
        }
    }
    if let Some(entry) = nearest.get_mut(a) {
        *entry = table.nearest(a, active);
    }
}

/// Moves every member of flat cluster `from` into `into`.
fn relabel(flat: &mut [usize], from: usize, into: usize) {
    let (old, new) = (flat.get(from).copied(), flat.get(into).copied());
    let (Some(old), Some(new)) = (old, new) else {
        return;
    };
    for label in flat.iter_mut().filter(|label| **label == old) {
        *label = new;
    }
}

/// Labels `0..k` in order of first appearance.
fn renumber(flat: &[usize]) -> Vec<usize> {
    let mut seen: Vec<usize> = Vec::new();
    flat.iter()
        .map(|label| {
            seen.iter().position(|s| s == label).unwrap_or_else(|| {
                seen.push(*label);
                seen.len() - 1
            })
        })
        .collect()
}

/// Squared euclidean distances between clusters, condensed (`i < j`).
struct Table {
    count: usize,
    d2: Vec<f64>,
}

impl Table {
    fn new(unit: &[Vec<f64>]) -> Self {
        let count = unit.len();
        let mut d2 = Vec::with_capacity(count * count.saturating_sub(1) / 2);
        for (i, x) in unit.iter().enumerate() {
            for y in unit.iter().skip(i + 1) {
                d2.push(x.iter().zip(y).map(|(p, q)| (p - q) * (p - q)).sum());
            }
        }
        Self { count, d2 }
    }

    fn index(&self, i: usize, j: usize) -> usize {
        let (i, j) = (i.min(j), i.max(j));
        i * self.count - i * (i + 1) / 2 + (j - i - 1)
    }

    fn get(&self, i: usize, j: usize) -> f64 {
        if i == j {
            return 0.0;
        }
        self.d2
            .get(self.index(i, j))
            .copied()
            .unwrap_or(f64::INFINITY)
    }

    fn set(&mut self, i: usize, j: usize, value: f64) {
        let at = self.index(i, j);
        if let Some(slot) = self.d2.get_mut(at) {
            *slot = value;
        }
    }

    /// Lance–Williams for centroids: `a ∪ b`'s squared distance to each `m`.
    fn merge(&mut self, (a, b): (usize, usize), (na, nb): (f64, f64), active: &[bool]) {
        let total = na + nb;
        let ab = self.get(a, b);
        for m in 0..self.count {
            if m == a || m == b || active.get(m) != Some(&true) {
                continue;
            }
            let value = (na * self.get(a, m) + nb * self.get(b, m)) / total
                - na * nb * ab / (total * total);
            self.set(a, m, value);
        }
    }

    /// `i`'s nearest active cluster and the squared distance to it.
    fn nearest(&self, i: usize, active: &[bool]) -> (f64, usize) {
        (0..self.count)
            .filter(|&j| j != i && active.get(j) == Some(&true))
            .map(|j| (self.get(i, j), j))
            .min_by(|x, y| x.0.total_cmp(&y.0))
            .unwrap_or((f64::INFINITY, i))
    }
}
