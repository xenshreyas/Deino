//! Message framing: `[u32 json_len][json][u32 pcm_len][pcm f32 LE]`.
//!
//! Lengths are little-endian byte counts. JSON (not a binary format) keeps
//! NaN-free optional confidences and enums readable in a hex dump, and the
//! PCM travels raw. A garbage length is refused before it becomes a huge
//! allocation. Ported from Handy's `engine_supervisor/protocol.rs` (MIT).

use std::io::{self, Read, Write};

use serde::Serialize;
use serde::de::DeserializeOwned;

/// The largest section a frame may carry: about an hour of 16 kHz PCM.
pub const MAX_SECTION_BYTES: usize = 1 << 28;

/// Bytes per PCM sample on the wire.
const SAMPLE_BYTES: usize = 4;

/// Encodes one frame. Refuses sections the reader would refuse, so an
/// oversized request fails here instead of killing the worker.
pub fn encode<T: Serialize>(message: &T, pcm: &[f32]) -> io::Result<Vec<u8>> {
    let json = serde_json::to_vec(message).map_err(io::Error::other)?;
    let json_len = section_len(json.len())?;
    let pcm_len = section_len(pcm.len().saturating_mul(SAMPLE_BYTES))?;
    let mut frame = Vec::with_capacity(8 + json.len() + pcm.len() * SAMPLE_BYTES);
    frame.extend_from_slice(&json_len.to_le_bytes());
    frame.extend_from_slice(&json);
    frame.extend_from_slice(&pcm_len.to_le_bytes());
    for sample in pcm {
        frame.extend_from_slice(&sample.to_le_bytes());
    }
    Ok(frame)
}

/// Encodes and writes one frame, then flushes.
pub fn write<T: Serialize>(out: &mut impl Write, message: &T, pcm: &[f32]) -> io::Result<()> {
    out.write_all(&encode(message, pcm)?)?;
    out.flush()
}

/// Reads one frame. `Ok(None)` only on a clean end of input before a frame
/// starts; a frame cut off part-way is an `UnexpectedEof` error.
pub fn read<T: DeserializeOwned>(input: &mut impl Read) -> io::Result<Option<(T, Vec<f32>)>> {
    let Some(json_len) = read_first_len(input)? else {
        return Ok(None);
    };
    let json = read_section(input, json_len)?;
    let message = serde_json::from_slice(&json).map_err(io::Error::other)?;
    let pcm_len = read_len(input)?;
    let pcm = decode_pcm(&read_section(input, pcm_len)?)?;
    Ok(Some((message, pcm)))
}

/// Reads the first length prefix, telling a clean end of input from a cut.
fn read_first_len(input: &mut impl Read) -> io::Result<Option<u32>> {
    let mut len = [0_u8; 4];
    let mut filled = 0;
    while filled < len.len() {
        let Some(rest) = len.get_mut(filled..) else {
            break;
        };
        match input.read(rest) {
            Ok(0) if filled == 0 => return Ok(None),
            Ok(0) => return Err(io::ErrorKind::UnexpectedEof.into()),
            Ok(n) => filled += n,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => return Err(error),
        }
    }
    Ok(Some(u32::from_le_bytes(len)))
}

fn read_len(input: &mut impl Read) -> io::Result<u32> {
    let mut len = [0_u8; 4];
    input.read_exact(&mut len)?;
    Ok(u32::from_le_bytes(len))
}

fn read_section(input: &mut impl Read, len: u32) -> io::Result<Vec<u8>> {
    let len = usize::try_from(len).map_err(io::Error::other)?;
    section_len(len)?;
    let mut section = vec![0_u8; len];
    input.read_exact(&mut section)?;
    Ok(section)
}

fn decode_pcm(bytes: &[u8]) -> io::Result<Vec<f32>> {
    let (samples, rest) = bytes.as_chunks::<SAMPLE_BYTES>();
    if !rest.is_empty() {
        return Err(io::Error::other(
            "PCM section is not a whole number of samples",
        ));
    }
    Ok(samples
        .iter()
        .map(|bytes| f32::from_le_bytes(*bytes))
        .collect())
}

/// Checks a section length against [`MAX_SECTION_BYTES`] and the `u32` prefix.
fn section_len(len: usize) -> io::Result<u32> {
    if len > MAX_SECTION_BYTES {
        return Err(io::Error::other(format!(
            "frame section too large: {len} bytes"
        )));
    }
    u32::try_from(len).map_err(io::Error::other)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{BatchParams, Request};

    fn run_frame(pcm: &[f32]) -> Vec<u8> {
        encode(&Request::Run(BatchParams::default()), pcm).unwrap()
    }

    #[test]
    fn a_frame_round_trips_its_message_and_pcm() {
        let pcm = [0.0_f32, -1.0, 0.5, f32::MIN_POSITIVE];
        let frame = run_frame(&pcm);

        let (request, samples): (Request, Vec<f32>) = read(&mut frame.as_slice()).unwrap().unwrap();

        assert_eq!(request, Request::Run(BatchParams::default()));
        assert_eq!(samples, pcm);
    }

    #[test]
    fn a_clean_end_of_input_is_not_an_error() {
        let empty: &[u8] = &[];

        assert!(read::<Request>(&mut &*empty).unwrap().is_none());
    }

    #[test]
    fn a_frame_cut_anywhere_is_an_unexpected_eof() {
        let frame = run_frame(&[0.5; 8]);
        for cut in [1, 3, 6, frame.len() - 34, frame.len() - 1] {
            let partial = &frame[..cut];

            let error = read::<Request>(&mut &*partial).unwrap_err();

            assert_eq!(error.kind(), io::ErrorKind::UnexpectedEof, "cut at {cut}");
        }
    }

    #[test]
    fn an_oversized_length_prefix_is_refused_before_allocating() {
        let mut frame = Vec::new();
        frame.extend_from_slice(&u32::MAX.to_le_bytes());

        let error = read::<Request>(&mut frame.as_slice()).unwrap_err();

        assert!(error.to_string().contains("too large"));
    }

    #[test]
    fn the_section_limit_is_inclusive_and_fits_the_prefix() {
        assert!(section_len(MAX_SECTION_BYTES).is_ok());
        assert!(section_len(MAX_SECTION_BYTES + 1).is_err());
    }
}
