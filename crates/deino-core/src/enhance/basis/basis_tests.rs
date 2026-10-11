//! Regenerate prompts: each change to the inputs yields its reason, and
//! nothing else does.

use deino_domain::{Millis, PersonId};
use deino_testkit::at;

use super::*;
use crate::enhance::test_support::{line, material};

fn person(n: u128) -> PersonId {
    uuid::Uuid::from_u128(n).to_string().parse().unwrap()
}

fn written() -> Material {
    let mut material = material(
        Some("- v tired"),
        vec![line(1, 0, "Maya", "hi"), line(2, 10, "Shreyas", "hello")],
    );
    material.attendee_ids = vec![person(2), person(1)];
    material
}

#[test]
fn unchanged_inputs_give_no_reason() {
    let basis = EnhanceBasis::of(&written());

    assert_eq!(basis.reasons(&written()), []);
    assert_eq!(
        basis.attendees,
        [person(1), person(2)],
        "attendees are sorted"
    );
}

#[test]
fn a_new_attendee_is_a_reason() {
    let basis = EnhanceBasis::of(&written());
    let mut now = written();
    now.attendee_ids.push(person(3));

    assert_eq!(basis.reasons(&now), [RegenerateReason::AttendeesChanged]);
}

#[test]
fn a_deleted_or_edited_segment_is_a_reason() {
    let basis = EnhanceBasis::of(&written());
    let mut deleted = written();
    deleted.transcript.remove(0);
    let mut edited = written();
    edited.transcript_edited_at = Some(at("2026-10-08T09:00:00Z"));

    assert_eq!(
        basis.reasons(&deleted),
        [RegenerateReason::TranscriptEdited]
    );
    assert_eq!(basis.reasons(&edited), [RegenerateReason::TranscriptEdited]);
}

#[test]
fn notes_edited_after_the_notes_were_written_are_a_reason() {
    let basis = EnhanceBasis::of(&written());
    let mut now = written();
    if let Some(notes) = now.notes.as_mut() {
        notes.updated_at = at("2026-10-07T18:00:00Z");
    }

    assert_eq!(basis.reasons(&now), [RegenerateReason::NotesEdited]);
}

#[test]
fn appended_audio_is_a_reason_but_not_a_transcript_edit() {
    let basis = EnhanceBasis::of(&written());
    let mut now = written();
    now.transcript.push(line(3, 60, "Maya", "one more thing"));

    assert_eq!(basis.reasons(&now), [RegenerateReason::AudioAppended]);
    assert_eq!(EnhanceBasis::of(&now).transcript_end, Millis(64_000));
}

#[test]
fn the_basis_round_trips_through_its_stored_json() {
    let basis = EnhanceBasis::of(&written());

    let json = serde_json::to_string(&basis).unwrap();

    assert_eq!(serde_json::from_str::<EnhanceBasis>(&json).unwrap(), basis);
    assert!(json.contains("segmentsDigest"));
}
