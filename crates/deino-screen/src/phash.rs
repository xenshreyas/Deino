//! [`PHash`]: a 64-bit perceptual hash of a frame, and the distance between
//! two of them.
//!
//! The classic DCT hash: the frame's luma averaged down to 32 × 32, a 2-D
//! DCT-II, the 8 × 8 lowest frequencies without the DC term, and one bit
//! per coefficient: set when it is above their median. Two frames of the
//! same slide hash a few bits apart (cursor, a clock, compression noise);
//! a new slide differs in dozens. It is size-independent, so a resized
//! window does not count as a change. Pure arithmetic, no allocation
//! beyond two small grids.

use std::f32::consts::PI;

use crate::frame::Frame;

/// The side of the averaged grid.
const SIDE: usize = 32;
/// The side of the kept low-frequency block.
const LOW: usize = 8;

/// A frame's perceptual hash.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PHash(pub u64);

impl PHash {
    /// The hash of `frame`.
    pub fn of(frame: &Frame) -> Self {
        let grid = luma_grid(frame);
        let low = low_frequencies(&grid);
        Self(bits_above_median(&low))
    }

    /// How many of the 64 bits differ: 0 for the same picture.
    pub const fn distance(self, other: Self) -> u32 {
        (self.0 ^ other.0).count_ones()
    }

    /// The hash as SQLite's signed 64-bit integer (the same bits).
    pub const fn to_i64(self) -> i64 {
        i64::from_ne_bytes(self.0.to_ne_bytes())
    }

    /// The hash from SQLite's signed 64-bit integer.
    pub const fn from_i64(value: i64) -> Self {
        Self(u64::from_ne_bytes(value.to_ne_bytes()))
    }
}

/// The frame's luma (0–255) averaged into a `SIDE × SIDE` grid, row-major.
fn luma_grid(frame: &Frame) -> Vec<f32> {
    let (width, height) = (frame.width(), frame.height());
    let mut grid = Vec::with_capacity(SIDE * SIDE);
    for row in 0..SIDE {
        let (top, bottom) = span(row, height);
        for column in 0..SIDE {
            let (left, right) = span(column, width);
            grid.push(average_luma(frame, (left, right), (top, bottom)));
        }
    }
    grid
}

/// The pixel range `[start, end)` that cell `index` of `SIDE` covers along
/// a side of `length` pixels; at least one pixel, even on tiny frames.
fn span(index: usize, length: u32) -> (u32, u32) {
    let (index, cells, length64) = (index as u64, SIDE as u64, u64::from(length));
    let start = u32::try_from(index * length64 / cells).unwrap_or(length);
    let end = u32::try_from((index + 1) * length64 / cells).unwrap_or(length);
    let start = start.min(length.saturating_sub(1));
    (start, end.max(start + 1))
}

/// The mean luma of the pixels in `[left, right) × [top, bottom)`.
fn average_luma(frame: &Frame, (left, right): (u32, u32), (top, bottom): (u32, u32)) -> f32 {
    let mut sum = 0.0_f32;
    for y in top..bottom {
        for x in left..right {
            let [b, g, r, _] = frame.pixel(x, y);
            sum += 0.114 * f32::from(b) + 0.587 * f32::from(g) + 0.299 * f32::from(r);
        }
    }
    let count = (right - left) as usize * (bottom - top) as usize;
    sum / small(count)
}

/// The `LOW × LOW` lowest DCT-II coefficients of `grid`, row-major,
/// computed separably (rows, then columns), only for the kept block.
fn low_frequencies(grid: &[f32]) -> Vec<f32> {
    let basis = cosine_basis();
    // Along each row: SIDE rows × LOW horizontal frequencies.
    let rows: Vec<[f32; LOW]> = grid
        .chunks(SIDE)
        .map(|row| {
            let mut out = [0.0_f32; LOW];
            for (slot, weights) in out.iter_mut().zip(basis.chunks(SIDE)) {
                *slot = dot(row, weights);
            }
            out
        })
        .collect();
    // Down each column of that: LOW vertical × LOW horizontal frequencies.
    let mut low = Vec::with_capacity(LOW * LOW);
    for weights in basis.chunks(SIDE) {
        let mut out = [0.0_f32; LOW];
        for (row, weight) in rows.iter().zip(weights) {
            for (slot, value) in out.iter_mut().zip(row) {
                *slot += value * weight;
            }
        }
        low.extend_from_slice(&out);
    }
    low
}

/// `cos((2x + 1) · k · π / 2N)` for the `LOW` kept frequencies `k`, each
/// row holding the `SIDE` samples `x`.
fn cosine_basis() -> Vec<f32> {
    let n = 2.0 * small(SIDE);
    let mut basis = Vec::with_capacity(LOW * SIDE);
    for k in 0..LOW {
        for x in 0..SIDE {
            let phase = small(2 * x + 1) * small(k) * PI / n;
            basis.push(phase.cos());
        }
    }
    basis
}

/// A small index as `f32` (exact: every value here is below 2^24).
#[allow(
    clippy::cast_precision_loss,
    reason = "grid indices and pixel counts are far below 2^24"
)]
const fn small(value: usize) -> f32 {
    value as f32
}

fn dot(left: &[f32], right: &[f32]) -> f32 {
    left.iter().zip(right).map(|(a, b)| a * b).sum()
}

/// One bit per coefficient after the DC term: set when above the median.
fn bits_above_median(low: &[f32]) -> u64 {
    let ac = low.get(1..).unwrap_or_default();
    let mut sorted = ac.to_vec();
    sorted.sort_by(f32::total_cmp);
    let median = sorted.get(sorted.len() / 2).copied().unwrap_or_default();
    ac.iter()
        .enumerate()
        .filter(|(_, value)| **value > median)
        .fold(0_u64, |bits, (index, _)| bits | (1 << index))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A frame with a white rectangle at `[x0, x1) × [y0, y1)` on black.
    fn slide(width: u32, height: u32, (x0, x1): (u32, u32), (y0, y1): (u32, u32)) -> Frame {
        let mut bgra = Vec::new();
        for y in 0..height {
            for x in 0..width {
                let lit = (x0..x1).contains(&x) && (y0..y1).contains(&y);
                bgra.extend(if lit {
                    [255, 255, 255, 255]
                } else {
                    [0, 0, 0, 255]
                });
            }
        }
        Frame::new(width, height, bgra).unwrap()
    }

    #[test]
    fn the_same_picture_hashes_the_same() {
        let frame = slide(320, 180, (40, 200), (20, 90));
        assert_eq!(PHash::of(&frame).distance(PHash::of(&frame.clone())), 0);
    }

    #[test]
    fn a_resized_window_is_not_a_change() {
        let small = slide(320, 180, (40, 200), (20, 90));
        let large = slide(640, 360, (80, 400), (40, 180));
        assert!(PHash::of(&small).distance(PHash::of(&large)) <= 4);
    }

    #[test]
    fn a_small_change_stays_close_and_a_new_slide_is_far() {
        let base = slide(320, 180, (40, 200), (20, 90));
        let cursor = {
            let mut bgra = base.bgra().to_vec();
            let at = (170 * 320 + 300) * 4;
            bgra[at..at + 4].copy_from_slice(&[255, 255, 255, 255]);
            Frame::new(320, 180, bgra).unwrap()
        };
        let next = slide(320, 180, (180, 300), (100, 170));

        assert!(PHash::of(&base).distance(PHash::of(&cursor)) <= 2);
        assert!(PHash::of(&base).distance(PHash::of(&next)) > 20);
    }

    #[test]
    fn tiny_frames_still_hash() {
        let frame = Frame::solid(3, 2, [10, 20, 30, 255]).unwrap();
        assert_eq!(PHash::of(&frame).distance(PHash::of(&frame)), 0);
    }

    #[test]
    fn the_sqlite_form_keeps_every_bit() {
        let hash = PHash(u64::MAX - 5);
        assert_eq!(PHash::from_i64(hash.to_i64()), hash);
    }
}
