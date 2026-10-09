//! Box scanner tests: clean files, torn tails, missing `moov`, odd headers.

use super::*;
use crate::fmp4_fixture::{init_segment, media_segment, mp4_box, track_file};

/// 47 packets ≈ one second of AAC at 48 kHz, what a CMAF segment holds.
const SECOND: u32 = 47;

fn write(bytes: &[u8]) -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("mic.001.m4a.part");
    std::fs::write(&path, bytes).unwrap();
    (dir, path)
}

#[test]
fn a_clean_file_is_all_playable() {
    let bytes = track_file(&[SECOND, SECOND, SECOND]);
    let (_dir, path) = write(&bytes);

    let found = scan(&path).unwrap();

    assert!(found.has_moov);
    assert_eq!(found.fragments, 3);
    assert_eq!(found.packets, 3 * 47);
    assert_eq!(found.valid_len, found.file_len);
    assert_eq!(found.playable_frames(), Frames(3 * 47 * 1024 - 2112));
}

#[test]
fn a_tail_torn_at_any_byte_is_cut_back_to_the_last_whole_fragment() {
    let whole = track_file(&[SECOND, SECOND]);
    let two_fragments_end = whole.len();
    let mut bytes = whole;
    bytes.extend(media_segment(SECOND));
    let torn_points = [
        two_fragments_end + 3,
        two_fragments_end + 9,
        bytes.len() - 1,
    ];

    for cut in torn_points {
        let (_dir, path) = write(&bytes[..cut]);

        let found = truncate_to_playable(&path).unwrap();

        assert_eq!(found.fragments, 2, "cut at {cut}");
        assert_eq!(found.valid_len, Bytes(two_fragments_end as u64));
        assert_eq!(
            std::fs::metadata(&path).unwrap().len(),
            two_fragments_end as u64
        );
    }
}

#[test]
fn truncation_is_idempotent() {
    let mut bytes = track_file(&[SECOND]);
    bytes.extend_from_slice(&media_segment(SECOND)[..30]);
    let (_dir, path) = write(&bytes);

    let first = truncate_to_playable(&path).unwrap();
    let second = truncate_to_playable(&path).unwrap();

    assert!(first.torn_bytes() > Bytes(0));
    assert_eq!(second.torn_bytes(), Bytes(0));
    assert_eq!(second.packets, first.packets);
}

#[test]
fn a_moof_without_its_mdat_is_not_counted() {
    let mut bytes = track_file(&[SECOND]);
    let segment = media_segment(SECOND);
    let moof_len = 8 + 8 + 8 + 8;
    bytes.extend_from_slice(&segment[..moof_len]);
    let (_dir, path) = write(&bytes);

    let found = scan(&path).unwrap();

    assert_eq!(found.fragments, 1);
    assert_eq!(found.file_len.0 - found.valid_len.0, moof_len as u64);
}

#[test]
fn a_truncated_moov_leaves_nothing_playable_and_the_file_untouched() {
    let init = init_segment();
    let (_dir, path) = write(&init[..init.len() - 10]);

    let found = truncate_to_playable(&path).unwrap();

    assert!(!found.has_moov);
    assert_eq!(found.playable_frames(), Frames::ZERO);
    assert_eq!(std::fs::metadata(&path).unwrap().len(), found.file_len.0);
}

#[test]
fn an_empty_file_scans_as_empty() {
    let (_dir, path) = write(&[]);
    assert_eq!(scan(&path).unwrap(), FragScan::default());
}

#[test]
fn a_box_claiming_to_run_to_end_of_file_ends_the_scan() {
    let mut bytes = init_segment();
    bytes.extend_from_slice(&[0, 0, 0, 0]);
    bytes.extend_from_slice(b"mdat");
    bytes.extend_from_slice(&[1; 32]);
    let (_dir, path) = write(&bytes);

    let found = scan(&path).unwrap();

    assert_eq!(found.valid_len, Bytes(init_segment().len() as u64));
}

#[test]
fn a_large_size_header_is_followed() {
    let mut bytes = init_segment();
    let free_body = [0u8; 4];
    bytes.extend_from_slice(&1u32.to_be_bytes());
    bytes.extend_from_slice(b"free");
    bytes.extend_from_slice(&(16 + free_body.len() as u64).to_be_bytes());
    bytes.extend_from_slice(&free_body);
    bytes.extend(media_segment(SECOND));
    let (_dir, path) = write(&bytes);

    let found = scan(&path).unwrap();

    assert_eq!(found.fragments, 1);
    assert_eq!(found.valid_len, found.file_len);
}

#[test]
fn media_packets_counts_every_trun_in_a_segment() {
    let mut segment = media_segment(10);
    segment.extend(media_segment(7));
    assert_eq!(media_packets(&segment), 17);
    assert_eq!(media_packets(&init_segment()), 0);
    assert_eq!(media_packets(&mp4_box(*b"moof", &[1, 2, 3])), 0);
}
