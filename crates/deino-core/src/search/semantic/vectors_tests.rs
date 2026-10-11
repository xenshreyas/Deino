use deino_domain::MeetingId;

use super::*;

fn stored(id: i64, meeting: &str, vector: &[f32]) -> StoredVector {
    StoredVector {
        id,
        meeting: MeetingId::new(meeting),
        vector: vector.to_vec(),
    }
}

fn index() -> VectorIndex {
    VectorIndex::new(vec![
        stored(1, "m1", &[1.0, 0.0, 0.0]),
        stored(2, "m1", &[0.0, 2.0, 0.0]),
        stored(3, "m2", &[3.0, 3.0, 0.0]),
        stored(4, "m3", &[0.0, 0.0, 5.0]),
    ])
}

fn ids(results: &[(i64, f32)]) -> Vec<i64> {
    results.iter().map(|(id, _)| *id).collect()
}

#[test]
fn search_ranks_rows_by_cosine_similarity_whatever_their_length() {
    let results = index().search(&[10.0, 0.0, 0.0], 3, |_| true);

    assert_eq!(ids(&results), [1, 3, 2]);
    assert!((results[0].1 - 1.0).abs() < 1e-6);
    assert!((results[1].1 - std::f32::consts::FRAC_1_SQRT_2).abs() < 1e-6);
}

#[test]
fn search_keeps_only_allowed_meetings_before_taking_the_top_k() {
    let results = index().search(&[1.0, 0.0, 0.0], 1, |meeting| meeting.as_str() != "m1");

    assert_eq!(ids(&results), [3]);
}

#[test]
fn removing_and_replacing_a_meeting_keeps_the_other_rows_intact() {
    let mut index = index();

    index.replace_meeting(
        &MeetingId::new("m1"),
        vec![stored(9, "m1", &[0.0, 0.0, -1.0])],
    );

    assert_eq!(index.len(), 3);
    assert_eq!(ids(&index.search(&[1.0, 1.0, 0.0], 1, |_| true)), [3]);
    assert_eq!(ids(&index.search(&[0.0, 0.0, 1.0], 3, |_| true)), [4, 3, 9]);
    index.remove_meeting(&MeetingId::new("m2"));
    index.remove_meeting(&MeetingId::new("m3"));
    assert_eq!(ids(&index.search(&[0.0, 0.0, -1.0], 5, |_| true)), [9]);
}

#[test]
fn vectors_of_another_length_and_mismatched_queries_are_refused() {
    let mut index = index();

    assert!(!index.insert(stored(5, "m4", &[1.0, 2.0])));
    assert!(!index.insert(stored(6, "m4", &[])));
    assert_eq!(index.len(), 4);
    assert!(index.search(&[1.0, 0.0], 3, |_| true).is_empty());
    assert!(index.search(&[1.0, 0.0, 0.0], 0, |_| true).is_empty());
}

#[test]
fn an_empty_index_finds_nothing() {
    let index = VectorIndex::default();

    assert!(index.is_empty());
    assert!(index.search(&[1.0], 3, |_| true).is_empty());
}
