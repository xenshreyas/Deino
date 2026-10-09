//! Synthetic fragmented MP4 files for scanner and recovery tests.
//!
//! The boxes carry only what the scanner reads (box sizes, types and
//! `trun.sample_count`), so torn-tail and missing-`moov` faults can be made
//! byte-exactly without running an encoder.

/// One box: 32-bit size, type, body.
pub(crate) fn mp4_box(kind: [u8; 4], body: &[u8]) -> Vec<u8> {
    let size = u32::try_from(body.len() + 8).unwrap();
    let mut bytes = size.to_be_bytes().to_vec();
    bytes.extend_from_slice(&kind);
    bytes.extend_from_slice(body);
    bytes
}

/// The init segment: `ftyp` + `moov`.
pub(crate) fn init_segment() -> Vec<u8> {
    let mut bytes = mp4_box(*b"ftyp", b"cmfc\0\0\0\0");
    bytes.extend(mp4_box(*b"moov", &[0; 64]));
    bytes
}

/// One media segment: a `moof` whose `trun` declares `packets` samples, and
/// an `mdat` with 200 bytes per packet.
pub(crate) fn media_segment(packets: u32) -> Vec<u8> {
    let mut trun = vec![0, 0, 0, 0];
    trun.extend_from_slice(&packets.to_be_bytes());
    let traf = mp4_box(*b"traf", &mp4_box(*b"trun", &trun));
    let mut bytes = mp4_box(*b"moof", &traf);
    let payload = vec![0xAA; usize::try_from(packets).unwrap() * 200];
    bytes.extend(mp4_box(*b"mdat", &payload));
    bytes
}

/// A whole track file: the init segment then one media segment per entry.
pub(crate) fn track_file(segments: &[u32]) -> Vec<u8> {
    let mut bytes = init_segment();
    for packets in segments {
        bytes.extend(media_segment(*packets));
    }
    bytes
}
