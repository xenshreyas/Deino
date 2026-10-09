//! Fragmented MP4 box scanning: counting packets and cutting torn tails.
//!
//! A track file is `ftyp` + `moov` (the init segment) followed by one
//! `moof` + `mdat` pair per one-second media segment. After `kill -9` the
//! file ends on a segment boundary, because each segment is one `write(2)`.
//! After a power loss the tail may be torn mid-box, which breaks some readers
//! (symphonia's probe), so recovery keeps only the longest prefix made of the
//! init segment and whole fragments ([`truncate_to_playable`]).
//!
//! Packet counts come from the `trun` sample counts (`moof → traf → trun`);
//! one AAC packet is [`AAC_FRAMES_PER_PACKET`] frames, and the first
//! [`AAC_PRIMING`] frames are encoder priming that readers trim.

use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;

use crate::durability::full_fsync;
use crate::units::{Bytes, Frames};

/// Frames in one AAC-LC packet.
pub const AAC_FRAMES_PER_PACKET: u64 = 1024;

/// Encoder priming Apple's AAC-LC encoder adds at the start of a stream; the
/// file's edit list tells `AVFoundation` and `ExtAudioFile` to skip it (symphonia
/// 0.5 ignores it, so never decode Deino's files with symphonia untrimmed).
pub const AAC_PRIMING: Frames = Frames(2112);

/// Bytes in a compact box header (32-bit size + type).
const HEADER_LEN: u64 = 8;

/// Bytes in an extended box header (size 1 + type + 64-bit size).
const LARGE_HEADER_LEN: u64 = 16;

/// A four-character box type.
type BoxType = [u8; 4];

/// What a scan of a fragmented MP4 file found.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FragScan {
    /// The file's length on disk.
    pub file_len: Bytes,
    /// The length of the playable prefix; anything after it is a torn tail.
    pub valid_len: Bytes,
    /// Complete `moof` + `mdat` fragments.
    pub fragments: u32,
    /// AAC packets in complete fragments.
    pub packets: u64,
    /// Whether a complete `moov` (init segment) was found. Without it the
    /// file holds no playable audio.
    pub has_moov: bool,
}

impl FragScan {
    /// Encoded frames in complete fragments, priming included.
    pub fn encoded_frames(&self) -> Frames {
        Frames(self.packets * AAC_FRAMES_PER_PACKET)
    }

    /// Frames a reader will decode once priming is trimmed.
    pub fn playable_frames(&self) -> Frames {
        self.encoded_frames() - AAC_PRIMING
    }

    /// Bytes past the playable prefix.
    pub fn torn_bytes(&self) -> Bytes {
        Bytes(self.file_len.0.saturating_sub(self.valid_len.0))
    }
}

/// Scans the top-level boxes of the file at `path`.
pub fn scan(path: &Path) -> io::Result<FragScan> {
    let mut file = File::open(path)?;
    let len = file.metadata()?.len();
    scan_reader(&mut file, len)
}

/// Cuts a torn tail off the file at `path`, so every reader sees a clean
/// file, and syncs the result. A file without a complete `moov` is left
/// untouched: it has nothing playable to keep.
pub fn truncate_to_playable(path: &Path) -> io::Result<FragScan> {
    let scanned = scan(path)?;
    if scanned.has_moov && scanned.valid_len < scanned.file_len {
        let file = OpenOptions::new().write(true).open(path)?;
        file.set_len(scanned.valid_len.0)?;
        full_fsync(&file)?;
    }
    Ok(scanned)
}

/// AAC packets in the `moof` boxes of an in-memory segment (what the segment
/// delegate counts as it appends a segment).
pub fn media_packets(segment: &[u8]) -> u64 {
    let mut packets = 0;
    for_each_child(segment, |kind, body| {
        if kind == b"moof" {
            packets += trun_samples(body);
        }
    });
    packets
}

/// Scans `len` bytes of boxes from `reader`.
fn scan_reader<R: Read + Seek>(reader: &mut R, len: u64) -> io::Result<FragScan> {
    let mut found = FragScan {
        file_len: Bytes(len),
        ..FragScan::default()
    };
    let (mut pos, mut pending_packets) = (0, None);
    while let Some((size, kind)) = box_header(reader, pos, len)? {
        let end = pos + size;
        match &kind {
            b"moov" => {
                found.has_moov = true;
                found.valid_len = Bytes(end);
            }
            b"moof" => pending_packets = Some(moof_packets(reader, pos, size)?),
            b"mdat" => {
                if let Some(packets) = pending_packets.take() {
                    found.fragments += 1;
                    found.packets += packets;
                    found.valid_len = Bytes(end);
                }
            }
            _ if !found.has_moov => found.valid_len = Bytes(end),
            _ => {}
        }
        pos = end;
    }
    Ok(found)
}

/// Reads the box header at `pos`: `(size, type)`, or `None` when the header
/// or the box itself runs past `len` (a torn tail) or is malformed.
fn box_header<R: Read + Seek>(
    reader: &mut R,
    pos: u64,
    len: u64,
) -> io::Result<Option<(u64, BoxType)>> {
    if pos + HEADER_LEN > len {
        return Ok(None);
    }
    let mut header = [0; 8];
    reader.seek(SeekFrom::Start(pos))?;
    reader.read_exact(&mut header)?;
    let [s0, s1, s2, s3, t0, t1, t2, t3] = header;
    let size = match u32::from_be_bytes([s0, s1, s2, s3]) {
        1 if pos + LARGE_HEADER_LEN <= len => {
            let mut large = [0; 8];
            reader.read_exact(&mut large)?;
            u64::from_be_bytes(large)
        }
        // 0 means "to the end of the file", which a finished fragment never uses.
        0 | 1 => return Ok(None),
        size => u64::from(size),
    };
    let fits = size >= HEADER_LEN && pos.checked_add(size).is_some_and(|end| end <= len);
    Ok(fits.then_some((size, [t0, t1, t2, t3])))
}

/// Reads a whole `moof` box and counts its packets.
fn moof_packets<R: Read + Seek>(reader: &mut R, pos: u64, size: u64) -> io::Result<u64> {
    let len = usize::try_from(size).map_err(|_| io::ErrorKind::InvalidData)?;
    let mut moof = vec![0; len];
    reader.seek(SeekFrom::Start(pos))?;
    reader.read_exact(&mut moof)?;
    Ok(media_packets(&moof))
}

/// Sums `trun.sample_count` over the `traf` boxes in a `moof` body.
fn trun_samples(moof_body: &[u8]) -> u64 {
    let mut samples = 0;
    for_each_child(moof_body, |kind, traf| {
        if kind == b"traf" {
            for_each_child(traf, |kind, trun| {
                // A full box: version (1 byte) and flags (3), then sample_count.
                if let (b"trun", Some(&[c0, c1, c2, c3])) = (kind, trun.get(4..8)) {
                    samples += u64::from(u32::from_be_bytes([c0, c1, c2, c3]));
                }
            });
        }
    });
    samples
}

/// Calls `visit(type, body)` for each complete compact box in `data`.
fn for_each_child(mut data: &[u8], mut visit: impl FnMut(&[u8], &[u8])) {
    while let Some(&[s0, s1, s2, s3]) = data.get(..4) {
        let size = usize::try_from(u32::from_be_bytes([s0, s1, s2, s3])).unwrap_or(0);
        let (Some(kind), Some(body)) = (data.get(4..8), data.get(8..size)) else {
            return;
        };
        visit(kind, body);
        data = data.get(size..).unwrap_or_default();
    }
}

#[cfg(test)]
#[path = "fmp4_tests.rs"]
mod tests;
