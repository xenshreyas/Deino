use deino_domain::{MeetingId, Millis};

use super::*;

/// A segment of `words` words (about 1.5 tokens per 6-character word).
fn line(index: usize, speaker: &str, words: usize) -> TranscriptLine {
    let start = u32::try_from(index).unwrap() * 1_000;
    TranscriptLine {
        segment_id: format!("s{index}"),
        start: Millis(start),
        end: Millis(start + 900),
        speaker: Some(speaker.to_owned()),
        text: vec!["word"; words].join(" ") + &format!(" end{index}"),
    }
}

#[test]
fn a_short_transcript_is_one_window_of_speaker_lines() {
    let lines = vec![line(0, "Maya", 3), line(1, "Maya", 2), line(2, "Tom", 2)];

    let chunks = transcript_chunks(&lines);

    assert_eq!(chunks.len(), 1);
    let chunk = &chunks[0];
    assert_eq!(chunk.kind, ChunkKind::Transcript);
    assert_eq!(chunk.ref_id, "s0");
    assert_eq!(chunk.span, Some((Millis(0), Millis(2_900))));
    assert_eq!(
        chunk.text,
        "Maya: word word word end0 word word end1\nTom: word word end2"
    );
}

#[test]
fn windows_stay_near_the_target_cut_at_turns_and_overlap_by_a_short_turn() {
    // 30 turns of about 63 tokens (50 words): six turns per window.
    let lines: Vec<TranscriptLine> = (0..30)
        .map(|index| line(index, if index % 2 == 0 { "Maya" } else { "Tom" }, 50))
        .collect();

    let chunks = transcript_chunks(&lines);

    assert!(chunks.len() >= 5, "{} windows", chunks.len());
    for (chunk, next) in chunks.iter().zip(chunks.iter().skip(1)) {
        assert!(estimate_tokens(&chunk.text) <= TARGET_TOKENS + 16);
        let last_turn = chunk.text.lines().last().unwrap();
        assert_eq!(next.text.lines().next().unwrap(), last_turn, "overlap");
    }
    let indexes: Vec<u32> = chunks.iter().map(|chunk| chunk.index).collect();
    assert_eq!(
        indexes,
        (0..u32::try_from(chunks.len()).unwrap()).collect::<Vec<_>>()
    );
    assert!(chunks.last().unwrap().text.ends_with("end29"));
}

#[test]
fn an_overlong_turn_is_split_between_its_segments_without_long_overlap() {
    // One speaker, 12 segments of about 100 tokens: no turn boundary at all.
    let lines: Vec<TranscriptLine> = (0..12).map(|index| line(index, "Maya", 80)).collect();

    let chunks = transcript_chunks(&lines);

    assert!(chunks.len() >= 3);
    let total_ends: usize = chunks
        .iter()
        .map(|chunk| chunk.text.matches(" end").count())
        .sum();
    assert!(total_ends >= 12, "every segment is in a window");
    for chunk in &chunks {
        assert!(estimate_tokens(&chunk.text) <= TARGET_TOKENS + 16);
    }
}

#[test]
fn every_segment_lands_in_some_window_in_order() {
    let lines: Vec<TranscriptLine> = (0..40)
        .map(|index| line(index, ["A", "B", "C"][index % 3], 5 + index % 7 * 20))
        .collect();

    let chunks = transcript_chunks(&lines);

    // Each segment's marker, in the order the windows hold them (overlaps
    // repeat a marker, never step back past the previous window's start).
    let seen: Vec<usize> = chunks
        .iter()
        .flat_map(|chunk| chunk.text.split_whitespace())
        .filter_map(|word| word.strip_prefix("end")?.parse().ok())
        .collect();
    let mut distinct = seen.clone();
    distinct.dedup();
    distinct.sort_unstable();
    distinct.dedup();
    assert_eq!(distinct, (0..40).collect::<Vec<usize>>());
    let firsts: Vec<usize> = chunks
        .iter()
        .filter_map(|chunk| {
            chunk
                .text
                .split_whitespace()
                .find_map(|word| word.strip_prefix("end")?.parse().ok())
        })
        .collect();
    assert!(firsts.is_sorted(), "{firsts:?}");
}

#[test]
fn an_empty_transcript_has_no_windows() {
    assert!(transcript_chunks(&[]).is_empty());
}

#[test]
fn notes_are_chunked_by_section() {
    let meeting = MeetingId::new("meeting-1");
    let markdown = "Intro line\n\n# Decisions\n- ship v2\n\n## Risks\n- Graph renewals\n\n#\n";

    let chunks = section_chunks(ChunkKind::Note, &meeting, markdown);

    let texts: Vec<&str> = chunks.iter().map(|chunk| chunk.text.as_str()).collect();
    assert_eq!(
        texts,
        [
            "Intro line",
            "# Decisions\n- ship v2",
            "## Risks\n- Graph renewals"
        ]
    );
    assert!(chunks.iter().all(|chunk| chunk.ref_id == "meeting-1"));
    assert!(chunks.iter().all(|chunk| chunk.span.is_none()));
}

#[test]
fn a_long_section_is_split_between_paragraphs() {
    let meeting = MeetingId::new("meeting-1");
    let paragraph = "word ".repeat(300);
    let markdown = format!("# Long\n\n{paragraph}\n\n{paragraph}\n\n{paragraph}");

    let chunks = section_chunks(ChunkKind::Summary, &meeting, &markdown);

    assert_eq!(chunks.len(), 3);
    assert!(chunks.iter().all(|chunk| chunk.kind == ChunkKind::Summary));
    assert!(chunks[0].text.starts_with("# Long"));
}

#[test]
fn chunks_are_capped_far_inside_the_model_context() {
    let meeting = MeetingId::new("meeting-1");
    let wall = "x".repeat(MAX_CHUNK_CHARS * 2);

    let chunks = section_chunks(ChunkKind::Note, &meeting, &wall);

    assert_eq!(chunks[0].text.chars().count(), MAX_CHUNK_CHARS);
}
