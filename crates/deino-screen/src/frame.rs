//! [`Frame`]: one captured picture of the meeting window, as owned 8-bit
//! BGRA pixels, and [`FrameRate`], how often the video takes one.
//!
//! `ScreenCaptureKit` hands frames over in `IOSurface`-backed pixel buffers
//! whose rows may be padded; the adapter copies them into a tightly packed
//! [`Frame`] ([`Frame::from_rows`]) so everything after it (the video
//! encoder, the perceptual hash, the PNG) is plain Rust and testable. At
//! the rates Deino captures (one frame a second by default) the copy costs
//! nothing worth measuring.

use std::fmt;

use serde::{Deserialize, Serialize};

/// Bytes per BGRA pixel.
const BYTES_PER_PIXEL: usize = 4;

/// Why pixels do not make a frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum FrameError {
    /// A side is zero.
    #[error("a frame needs a width and a height")]
    Empty,
    /// The bytes do not cover the rows the size and stride say.
    #[error("the pixel data is shorter than the frame")]
    Short,
}

/// One frame: `width × height` BGRA pixels, rows packed without padding.
#[derive(Clone, PartialEq, Eq)]
pub struct Frame {
    width: u32,
    height: u32,
    bgra: Vec<u8>,
}

impl Frame {
    /// A frame over packed BGRA bytes (`width × height × 4` of them).
    pub fn new(width: u32, height: u32, bgra: Vec<u8>) -> Result<Self, FrameError> {
        let len = packed_len(width, height)?;
        if bgra.len() < len {
            return Err(FrameError::Short);
        }
        let mut bgra = bgra;
        bgra.truncate(len);
        Ok(Self {
            width,
            height,
            bgra,
        })
    }

    /// Copies `height` rows of `stride` bytes each (padding dropped) out of
    /// `data`, a pixel buffer's base address.
    pub fn from_rows(
        width: u32,
        height: u32,
        stride: usize,
        data: &[u8],
    ) -> Result<Self, FrameError> {
        let row = packed_len(width, 1)?;
        let len = packed_len(width, height)?;
        if stride < row || data.len() < stride * (to_usize(height) - 1) + row {
            return Err(FrameError::Short);
        }
        let mut bgra = Vec::with_capacity(len);
        for line in data.chunks(stride).take(to_usize(height)) {
            bgra.extend_from_slice(line.get(..row).ok_or(FrameError::Short)?);
        }
        Self::new(width, height, bgra)
    }

    /// One colour everywhere, as `[b, g, r, a]` (tests and fakes).
    pub fn solid(width: u32, height: u32, bgra: [u8; 4]) -> Result<Self, FrameError> {
        let pixels = packed_len(width, height)? / BYTES_PER_PIXEL;
        Self::new(width, height, bgra.repeat(pixels))
    }

    /// The width in pixels.
    pub const fn width(&self) -> u32 {
        self.width
    }

    /// The height in pixels.
    pub const fn height(&self) -> u32 {
        self.height
    }

    /// The packed BGRA bytes, row by row.
    pub fn bgra(&self) -> &[u8] {
        &self.bgra
    }

    /// The bytes of one row.
    pub const fn stride(&self) -> usize {
        to_usize(self.width) * BYTES_PER_PIXEL
    }

    /// The pixel at (`x`, `y`) as `[b, g, r, a]`; black outside the frame.
    pub fn pixel(&self, column: u32, row: u32) -> [u8; 4] {
        let at = (to_usize(row) * to_usize(self.width) + to_usize(column)) * BYTES_PER_PIXEL;
        self.bgra
            .get(at..at + BYTES_PER_PIXEL)
            .and_then(|pixel| <[u8; 4]>::try_from(pixel).ok())
            .unwrap_or_default()
    }
}

impl fmt::Debug for Frame {
    /// The size only: a frame's pixels are the user's screen.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "Frame({}×{})", self.width, self.height)
    }
}

/// `u32` → `usize`, lossless on every platform Deino builds for.
const fn to_usize(value: u32) -> usize {
    value as usize
}

/// The packed byte length of `width × height` BGRA pixels.
fn packed_len(width: u32, height: u32) -> Result<usize, FrameError> {
    if width == 0 || height == 0 {
        return Err(FrameError::Empty);
    }
    Ok(to_usize(width) * to_usize(height) * BYTES_PER_PIXEL)
}

/// How many frames a second the video takes: 1 by default (a shared
/// screen changes slowly), at most [`FrameRate::MAX`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(transparent)]
pub struct FrameRate(u8);

impl FrameRate {
    /// The default: one frame a second, H.264 at about 250 kbps.
    pub const DEFAULT: Self = Self(1);
    /// The highest rate offered.
    pub const MAX: u8 = 30;

    /// `fps` frames a second, clamped to `1..=MAX`.
    pub const fn new(fps: u8) -> Self {
        if fps == 0 {
            Self(1)
        } else if fps > Self::MAX {
            Self(Self::MAX)
        } else {
            Self(fps)
        }
    }

    /// Frames per second.
    pub const fn fps(self) -> u8 {
        self.0
    }

    /// The H.264 bit rate for this rate: 250 kbps a frame per second,
    /// capped at 2.5 Mbps (about 110 MB an hour at the default).
    pub fn bit_rate(self) -> u32 {
        (u32::from(self.0) * 250_000).min(2_500_000)
    }
}

impl Default for FrameRate {
    fn default() -> Self {
        Self::DEFAULT
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_are_copied_without_their_padding() {
        // 2×2 pixels with 4 bytes of padding per row.
        let data = [
            1, 1, 1, 1, 2, 2, 2, 2, 9, 9, 9, 9, //
            3, 3, 3, 3, 4, 4, 4, 4, 9, 9, 9, 9,
        ];
        let frame = Frame::from_rows(2, 2, 12, &data).unwrap();

        assert_eq!(frame.bgra().len(), 16);
        assert_eq!(frame.pixel(1, 0), [2, 2, 2, 2]);
        assert_eq!(frame.pixel(0, 1), [3, 3, 3, 3]);
        assert_eq!(frame.pixel(5, 5), [0, 0, 0, 0]);
    }

    #[test]
    fn short_or_empty_pixels_are_refused() {
        assert_eq!(Frame::new(0, 4, vec![]), Err(FrameError::Empty));
        assert_eq!(Frame::new(2, 2, vec![0; 15]), Err(FrameError::Short));
        assert_eq!(Frame::from_rows(2, 2, 4, &[0; 64]), Err(FrameError::Short));
        assert_eq!(Frame::from_rows(2, 2, 8, &[0; 12]), Err(FrameError::Short));
    }

    #[test]
    fn a_frame_never_prints_its_pixels() {
        let frame = Frame::solid(3, 2, [0, 0, 0, 255]).unwrap();
        assert_eq!(format!("{frame:?}"), "Frame(3×2)");
    }

    #[test]
    fn rates_are_clamped_and_set_the_bit_rate() {
        assert_eq!(FrameRate::new(0).fps(), 1);
        assert_eq!(FrameRate::new(200).fps(), FrameRate::MAX);
        assert_eq!(FrameRate::DEFAULT.bit_rate(), 250_000);
        assert_eq!(FrameRate::new(30).bit_rate(), 2_500_000);
    }
}
