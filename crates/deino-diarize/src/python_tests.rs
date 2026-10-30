use super::*;

#[test]
fn voices_are_numbered_by_first_appearance_and_turns_point_at_them() {
    let answer: Diarized = serde_json::from_str(
        r#"{
            "turns": [
                {"start_ms": 5000, "end_ms": 7000, "speaker": "SPEAKER_00"},
                {"start_ms": 1000, "end_ms": 3000, "speaker": "SPEAKER_01"},
                {"start_ms": 8000, "end_ms": 8000, "speaker": "SPEAKER_01"},
                {"start_ms": 9000, "end_ms": 9500, "speaker": "SPEAKER_02"}
            ],
            "voices": {"SPEAKER_00": [1.0, 0.0], "SPEAKER_01": [0.0, 1.0]}
        }"#,
    )
    .unwrap();

    let diarization = answer.into_diarization();

    assert_eq!(
        diarization.voices.len(),
        2,
        "a voice without an embedding is dropped"
    );
    assert_eq!(
        diarization.voices[0].centroid,
        [0.0, 1.0],
        "SPEAKER_01 spoke first"
    );
    assert_eq!(diarization.voices[0].speech, Millis(2_000));
    let turns: Vec<(u32, usize)> = diarization
        .turns
        .iter()
        .map(|t| (t.start.0, t.voice))
        .collect();
    assert_eq!(turns, [(1_000, 0), (5_000, 1)], "empty turns are dropped");
}

#[test]
fn the_calendar_hint_bounds_the_speaker_count() {
    assert!(bound_args(SpeakerHint::Unknown).is_empty());
    assert_eq!(bound_args(SpeakerHint::AtMost(4)), ["--max-speakers", "4"]);
    assert_eq!(
        bound_args(SpeakerHint::Exactly(2)),
        ["--min-speakers", "2", "--max-speakers", "2"]
    );
}

#[test]
fn an_engine_is_installed_only_with_its_environment_and_script() {
    let root = tempfile::tempdir().unwrap();
    assert!(PythonEngine::installed(root.path()).is_none());
    std::fs::create_dir_all(root.path().join(".venv/bin")).unwrap();
    std::fs::write(root.path().join(".venv/bin/python"), "").unwrap();
    std::fs::write(root.path().join("speakers.py"), "").unwrap();
    assert!(PythonEngine::installed(root.path()).is_some());
}
