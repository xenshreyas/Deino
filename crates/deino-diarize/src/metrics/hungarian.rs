//! The Hungarian algorithm (Jonker–Volgenant style, O(n³)) for the optimal
//! one-to-one speaker mapping. Ported from anarlog's `metrics.rs` (MIT).

/// `(row, col)` pairs maximising the total weight of a rectangular matrix;
/// pairs with zero weight are left out.
pub(super) fn max_weight_matching(weights: &[Vec<f64>]) -> Vec<(usize, usize)> {
    let rows = weights.len();
    let cols = weights.first().map_or(0, Vec::len);
    if rows == 0 || cols == 0 {
        return Vec::new();
    }
    let weight = |r: usize, c: usize| weights.get(r).and_then(|row| row.get(c)).copied();
    let max = weights.iter().flatten().copied().fold(0.0, f64::max);
    // Minimise `max - weight` on a square matrix padded with `max`.
    let cost = |r: usize, c: usize| max - weight(r, c).unwrap_or(0.0);
    let assignment = solve(rows.max(cols), &cost);
    assignment
        .into_iter()
        .enumerate()
        .filter_map(|(col, row)| {
            let row = row?;
            (row < rows && col < cols && weight(row, col).is_some_and(|w| w > 0.0))
                .then_some((row, col))
        })
        .collect()
}

/// The potentials and matching of the shortest-augmenting-path solver.
/// Index 0 is the virtual start column; real rows and columns are 1-based.
struct Solver<'a> {
    cost: &'a dyn Fn(usize, usize) -> f64,
    row_potential: Vec<f64>,
    col_potential: Vec<f64>,
    /// `matched[c]`: the row matched to column `c` (0 = none).
    matched: Vec<usize>,
    /// `previous[c]`: the column before `c` on the augmenting path.
    previous: Vec<usize>,
}

/// For each column, the row assigned to it, minimising total `cost` on an
/// `n × n` matrix.
fn solve(n: usize, cost: &dyn Fn(usize, usize) -> f64) -> Vec<Option<usize>> {
    let mut solver = Solver {
        cost,
        row_potential: vec![0.0; n + 1],
        col_potential: vec![0.0; n + 1],
        matched: vec![0; n + 1],
        previous: vec![0; n + 1],
    };
    for row in 1..=n {
        solver.add_row(row, n);
    }
    (1..=n)
        .map(|col| get(&solver.matched, col).checked_sub(1))
        .collect()
}

impl Solver<'_> {
    /// Matches `row`, re-routing earlier matches along the cheapest path.
    fn add_row(&mut self, row: usize, n: usize) {
        set(&mut self.matched, 0, row);
        let mut col = 0;
        let mut slack = vec![f64::INFINITY; n + 1];
        let mut used = vec![false; n + 1];
        loop {
            set(&mut used, col, true);
            let (delta, next) = self.relax(col, &used, &mut slack);
            self.shift(delta, &used, &mut slack);
            col = next;
            if get(&self.matched, col) == 0 {
                break;
            }
        }
        while col != 0 {
            let before = get(&self.previous, col);
            let row = get(&self.matched, before);
            set(&mut self.matched, col, row);
            col = before;
        }
    }

    /// Updates the slack of every unused column from `col`'s row; returns
    /// the smallest slack and its column.
    fn relax(&mut self, col: usize, used: &[bool], slack: &mut [f64]) -> (f64, usize) {
        let row = get(&self.matched, col);
        let mut best = (f64::INFINITY, 0);
        for next in 1..used.len() {
            if get(used, next) {
                continue;
            }
            let reduced = (self.cost)(row.saturating_sub(1), next - 1)
                - get(&self.row_potential, row)
                - get(&self.col_potential, next);
            if reduced < get(slack, next) {
                set(slack, next, reduced);
                set(&mut self.previous, next, col);
            }
            if get(slack, next) < best.0 {
                best = (get(slack, next), next);
            }
        }
        best
    }

    /// Moves the potentials by `delta` so a new tight edge appears.
    fn shift(&mut self, delta: f64, used: &[bool], slack: &mut [f64]) {
        for col in 0..used.len() {
            if get(used, col) {
                let row = get(&self.matched, col);
                add(&mut self.row_potential, row, delta);
                add(&mut self.col_potential, col, -delta);
            } else {
                add(slack, col, -delta);
            }
        }
    }
}

fn get<T: Copy + Default>(values: &[T], index: usize) -> T {
    values.get(index).copied().unwrap_or_default()
}

fn add(values: &mut [f64], index: usize, delta: f64) {
    if let Some(slot) = values.get_mut(index) {
        *slot += delta;
    }
}

fn set<T>(values: &mut [T], index: usize, value: T) {
    if let Some(slot) = values.get_mut(index) {
        *slot = value;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_heaviest_pairing_wins_over_greedy_choices() {
        // Greedy would take (0, 0) = 5 and then (1, 1) = 1; optimal is 4 + 4.
        let weights = vec![vec![5.0, 4.0], vec![4.0, 1.0]];
        let mut pairs = max_weight_matching(&weights);
        pairs.sort_unstable();
        assert_eq!(pairs, vec![(0, 1), (1, 0)]);
    }

    #[test]
    fn rectangular_matrices_leave_extras_unmatched() {
        let weights = vec![vec![0.0, 3.0, 1.0]];
        assert_eq!(max_weight_matching(&weights), vec![(0, 1)]);
        let tall = vec![vec![2.0], vec![7.0], vec![0.0]];
        assert_eq!(max_weight_matching(&tall), vec![(1, 0)]);
    }

    #[test]
    fn zero_weights_and_empty_matrices_match_nothing() {
        assert!(max_weight_matching(&[]).is_empty());
        assert!(max_weight_matching(&[vec![0.0, 0.0]]).is_empty());
    }
}
