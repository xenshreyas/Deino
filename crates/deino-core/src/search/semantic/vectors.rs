//! The in-memory vector index: exact cosine search over every chunk.
//!
//! Vectors are kept unit length in one contiguous `f32` matrix, so a search
//! is one dot product per row, a loop the compiler vectorises. At personal
//! scale this is fast enough and exact: 40,000 chunks of 768 dimensions take
//! about 15 ms on an M-series Mac and 120 MB (ADR 0029). Rows are added and
//! removed a meeting at a time, as the indexer replaces a meeting's chunks.

use deino_domain::MeetingId;

/// A stored vector: its `embeddings` row, its meeting and its values.
#[derive(Clone, Debug, PartialEq)]
pub struct StoredVector {
    /// The `embeddings` row.
    pub id: i64,
    /// The meeting it belongs to.
    pub meeting: MeetingId,
    /// The vector (normalised on insert).
    pub vector: Vec<f32>,
}

/// Exact nearest-neighbour search by cosine similarity.
#[derive(Clone, Debug, Default)]
pub struct VectorIndex {
    dims: usize,
    ids: Vec<i64>,
    meetings: Vec<MeetingId>,
    matrix: Vec<f32>,
}

impl VectorIndex {
    /// An index of `vectors`. Vectors whose length differs from the first
    /// one's (another model's) are left out.
    pub fn new(vectors: Vec<StoredVector>) -> Self {
        let mut index = Self::default();
        for vector in vectors {
            index.insert(vector);
        }
        index
    }

    /// How many vectors it holds.
    pub fn len(&self) -> usize {
        self.ids.len()
    }

    /// Whether it holds nothing.
    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    /// Adds a vector, normalised. Returns false (and adds nothing) when it
    /// is empty or its length differs from the index's.
    pub fn insert(&mut self, stored: StoredVector) -> bool {
        if stored.vector.is_empty() || (self.dims != 0 && stored.vector.len() != self.dims) {
            return false;
        }
        self.dims = stored.vector.len();
        self.matrix.extend(normalised(&stored.vector));
        self.ids.push(stored.id);
        self.meetings.push(stored.meeting);
        true
    }

    /// Drops every vector of `meeting`, compacting the rows in one pass.
    pub fn remove_meeting(&mut self, meeting: &MeetingId) {
        let mut kept = 0;
        for row in 0..self.ids.len() {
            if self.meetings.get(row) == Some(meeting) {
                continue;
            }
            if kept != row {
                self.ids.swap(kept, row);
                self.meetings.swap(kept, row);
                let start = row * self.dims;
                self.matrix
                    .copy_within(start..start + self.dims, kept * self.dims);
            }
            kept += 1;
        }
        self.ids.truncate(kept);
        self.meetings.truncate(kept);
        self.matrix.truncate(kept * self.dims);
    }

    /// Replaces `meeting`'s vectors with `vectors`.
    pub fn replace_meeting(&mut self, meeting: &MeetingId, vectors: Vec<StoredVector>) {
        self.remove_meeting(meeting);
        for vector in vectors {
            self.insert(vector);
        }
    }

    /// The `k` rows most similar to `query`, best first, among the rows
    /// whose meeting `allow` accepts: `(row id, cosine similarity)`.
    pub fn search(
        &self,
        query: &[f32],
        k: usize,
        allow: impl Fn(&MeetingId) -> bool,
    ) -> Vec<(i64, f32)> {
        if query.len() != self.dims || k == 0 {
            return Vec::new();
        }
        let query = normalised(query);
        let mut scored: Vec<(i64, f32)> = self
            .matrix
            .chunks_exact(self.dims)
            .zip(self.ids.iter().zip(&self.meetings))
            .filter(|(_, (_, meeting))| allow(meeting))
            .map(|(row, (id, _))| (*id, dot(row, &query)))
            .collect();
        let by_score = |a: &(i64, f32), b: &(i64, f32)| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0));
        if scored.len() > k {
            scored.select_nth_unstable_by(k - 1, by_score);
            scored.truncate(k);
        }
        scored.sort_unstable_by(by_score);
        scored
    }
}

fn dot(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

/// `vector` scaled to unit length; a zero vector stays zero.
fn normalised(vector: &[f32]) -> Vec<f32> {
    let norm = dot(vector, vector).sqrt();
    if norm > 0.0 {
        vector.iter().map(|value| value / norm).collect()
    } else {
        vector.to_vec()
    }
}

#[cfg(test)]
#[path = "vectors_tests.rs"]
mod tests;
