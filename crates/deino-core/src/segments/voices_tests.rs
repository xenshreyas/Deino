//! The diarization writes' contract (`replace_speakers`, `voice`,
//! `reattribute`), run against the SQLite repository and the fake, plus
//! what only the real schema can check.

use deino_db::test_db;
use deino_domain::{Channel, FakeClock, MeetingId, Millis, PaletteColor, SegmentId, SpeakerId};
use deino_testkit::{at, seed};

use super::*;

const MEETING: &str = "meeting-a";

fn clock() -> FakeClock {
    FakeClock::new(at("2026-10-08T09:00:00Z"))
}

fn speaker(label: &str, is_me: bool) -> Speaker {
    Speaker {
        id: SpeakerId::generate(&clock()),
        meeting_id: MeetingId::new(MEETING),
        label: label.into(),
        person_id: None,
        is_me,
        color: PaletteColor::cycle(0),
    }
}

fn voiced(label: &str, centroid: Vec<f32>) -> NewSpeaker {
    NewSpeaker {
        speaker: speaker(label, false),
        voice: Some(SpeakerVoice {
            centroid,
            embedding_model: "wespeaker@test".into(),
            speech: Millis(4_200),
        }),
    }
}

fn word(text: &str, start: u32, end: u32) -> WordTiming {
    WordTiming {
        text: text.into(),
        start: Millis(start),
        end: Millis(end),
    }
}

fn segment(start: u32, words: Vec<WordTiming>) -> NewSegment {
    NewSegment {
        id: SegmentId::generate(&clock()),
        meeting_id: MeetingId::new(MEETING),
        channel: Channel::System,
        speaker_id: None,
        start: Millis(start),
        end: words.last().map_or(Millis(start + 1_000), |w| w.end),
        text: words
            .iter()
            .map(|w| w.text.as_str())
            .collect::<Vec<_>>()
            .join(" "),
        words,
    }
}

fn piece(speaker: Option<SpeakerId>, words: &[WordTiming]) -> SegmentPiece {
    SegmentPiece {
        speaker_id: speaker,
        start: words[0].start,
        end: words[words.len() - 1].end,
        text: words
            .iter()
            .map(|w| w.text.as_str())
            .collect::<Vec<_>>()
            .join(" "),
        words: words.to_vec(),
    }
}

async fn sqlite_repo() -> SqliteSegmentRepo {
    let database = test_db().await;
    for id in [MEETING, "meeting-b"] {
        seed::meeting(&database, id, "Standup", at("2026-10-08T09:00:00Z")).await;
    }
    SqliteSegmentRepo::new(database)
}

async fn labels(repo: &impl SpeakerRepo) -> Vec<String> {
    let speakers = repo.speakers(&MeetingId::new(MEETING)).await.unwrap();
    speakers.into_iter().map(|speaker| speaker.label).collect()
}

/// `(id, text, speaker, start, end)` of each listed segment.
type Row = (SegmentId, String, Option<SpeakerId>, Millis, Millis);

async fn rows(repo: &impl SegmentReader) -> Vec<Row> {
    let segments = repo
        .list(&MeetingId::new(MEETING), Page::ALL)
        .await
        .unwrap();
    let row = |s: Segment| (s.id, s.text, s.speaker_id, s.start, s.end);
    segments.into_iter().map(row).collect()
}

/// The user's speaker and a placeholder "Them" who said one segment.
async fn me_and_them(repo: &(impl SpeakerRepo + SegmentWriter)) -> (Speaker, Speaker) {
    let (me, them) = (speaker("Me", true), speaker("Them", false));
    repo.add_speaker(&me).await.unwrap();
    repo.add_speaker(&them).await.unwrap();
    let mut said = segment(0, vec![word("hi", 0, 400)]);
    said.speaker_id = Some(them.id);
    repo.append(&[said]).await.unwrap();
    (me, them)
}

/// Replacing keeps the user's speaker, swaps the rest, stores voices and
/// unlinks the segments of removed speakers.
async fn replacing_speakers_keeps_me_and_stores_voices(
    repo: &(impl SpeakerRepo + SegmentReader + SegmentWriter),
) {
    let meeting = MeetingId::new(MEETING);
    let (me, them) = me_and_them(repo).await;
    let first = voiced("Speaker 1", vec![0.6, -0.8, 1.0e-7]);
    let second = voiced("Speaker 2", vec![1.0]);

    repo.replace_speakers(&meeting, &[first.clone(), second])
        .await
        .unwrap();

    assert_eq!(labels(repo).await, ["Me", "Speaker 1", "Speaker 2"]);
    assert_eq!(repo.voice(&first.speaker.id).await.unwrap(), first.voice);
    assert_eq!(repo.voice(&me.id).await.unwrap(), None);
    assert_eq!(repo.voice(&them.id).await.unwrap(), None);
    assert_eq!(
        rows(repo).await[0].2,
        None,
        "the removed speaker's segment lost it"
    );
}

/// A second result replaces the first instead of adding to it.
async fn a_rerun_replaces_the_speakers(repo: &(impl SpeakerRepo + SegmentReader + SegmentWriter)) {
    let meeting = MeetingId::new(MEETING);
    me_and_them(repo).await;
    let twice = [
        voiced("Speaker 1", vec![1.0]),
        voiced("Speaker 2", vec![1.0]),
    ];
    repo.replace_speakers(&meeting, &twice).await.unwrap();

    repo.replace_speakers(&meeting, &[voiced("Speaker 1", vec![0.5])])
        .await
        .unwrap();

    assert_eq!(labels(repo).await, ["Me", "Speaker 1"]);
}

/// Two voiced speakers stored for the meeting; their IDs.
async fn two_speakers(repo: &impl SpeakerRepo) -> (SpeakerId, SpeakerId) {
    let speakers = [
        voiced("Speaker 1", vec![1.0]),
        voiced("Speaker 2", vec![1.0]),
    ];
    let ids = (speakers[0].speaker.id, speakers[1].speaker.id);
    repo.replace_speakers(&MeetingId::new(MEETING), &speakers)
        .await
        .unwrap();
    ids
}

const SAID: [(&str, u32, u32); 3] = [("yes", 0, 300), ("so", 400, 600), ("anyway", 700, 1_000)];

/// The head piece rewrites the segment in place; tail pieces become new
/// segments on its meeting and channel; a missing segment is skipped.
async fn reattribution_splits_segments_at_speaker_changes(
    repo: &(impl SpeakerRepo + SegmentReader + SegmentWriter),
) {
    let (a, b) = two_speakers(repo).await;
    let words: Vec<WordTiming> = SAID
        .iter()
        .map(|&(text, start, end)| word(text, start, end))
        .collect();
    let original = segment(0, words.clone());
    repo.append(std::slice::from_ref(&original)).await.unwrap();
    let tail = SegmentId::generate(&clock());
    let head = piece(Some(a), &words[..1]);
    let split = Reattribution {
        id: original.id,
        head,
        tail: vec![(tail, piece(Some(b), &words[1..]))],
    };
    let missing = Reattribution {
        id: SegmentId::generate(&clock()),
        head: piece(None, &words),
        tail: Vec::new(),
    };

    repo.reattribute(&[split, missing]).await.unwrap();

    let first = (
        original.id,
        "yes".to_owned(),
        Some(a),
        Millis(0),
        Millis(300),
    );
    let second = (
        tail,
        "so anyway".to_owned(),
        Some(b),
        Millis(400),
        Millis(1_000),
    );
    assert_eq!(rows(repo).await, vec![first, second]);
    assert_split_off_inherits(repo, &words[1..]).await;
}

/// The split-off piece sits on the original's channel with its own words.
async fn assert_split_off_inherits(repo: &impl SegmentReader, words: &[WordTiming]) {
    let listed = repo
        .list(&MeetingId::new(MEETING), Page::ALL)
        .await
        .unwrap();
    assert_eq!(listed[1].channel, Channel::System);
    assert_eq!(listed[1].words, words);
}

#[tokio::test]
async fn sqlite_replacing_speakers_keeps_me_and_stores_voices() {
    replacing_speakers_keeps_me_and_stores_voices(&sqlite_repo().await).await;
}

#[tokio::test]
async fn fake_replacing_speakers_keeps_me_and_stores_voices() {
    replacing_speakers_keeps_me_and_stores_voices(&FakeSegmentRepo::default()).await;
}

#[tokio::test]
async fn sqlite_a_rerun_replaces_the_speakers() {
    a_rerun_replaces_the_speakers(&sqlite_repo().await).await;
}

#[tokio::test]
async fn fake_a_rerun_replaces_the_speakers() {
    a_rerun_replaces_the_speakers(&FakeSegmentRepo::default()).await;
}

#[tokio::test]
async fn sqlite_reattribution_splits_segments_at_speaker_changes() {
    reattribution_splits_segments_at_speaker_changes(&sqlite_repo().await).await;
}

#[tokio::test]
async fn fake_reattribution_splits_segments_at_speaker_changes() {
    reattribution_splits_segments_at_speaker_changes(&FakeSegmentRepo::default()).await;
}

#[tokio::test]
async fn a_reattribution_to_another_meetings_speaker_rolls_back_the_whole_batch() {
    let repo = sqlite_repo().await;
    let mut stranger = voiced("Speaker 1", vec![1.0]);
    stranger.speaker.meeting_id = MeetingId::new("meeting-b");
    let other_meeting = MeetingId::new("meeting-b");
    repo.replace_speakers(&other_meeting, std::slice::from_ref(&stranger))
        .await
        .unwrap();
    let words = [word("one", 0, 300), word("two", 400, 700)];
    let (first, second) = (
        segment(0, words[..1].to_vec()),
        segment(400, words[1..].to_vec()),
    );
    repo.append(&[first.clone(), second.clone()]).await.unwrap();
    let fine = Reattribution {
        id: first.id,
        head: piece(None, &words[..1]),
        tail: Vec::new(),
    };
    let foreign = piece(Some(stranger.speaker.id), &words[1..]);
    let bad = Reattribution {
        id: second.id,
        head: foreign,
        tail: Vec::new(),
    };

    assert!(repo.reattribute(&[fine, bad]).await.is_err());

    let texts: Vec<String> = rows(&repo).await.into_iter().map(|row| row.1).collect();
    assert_eq!(texts, ["one", "two"], "nothing changed");
}

#[tokio::test]
async fn the_schema_rejects_an_empty_centroid() {
    let repo = sqlite_repo().await;
    let mut odd = voiced("Speaker 1", vec![1.0]);
    odd.voice.as_mut().unwrap().centroid.clear();
    let result = repo
        .replace_speakers(&MeetingId::new(MEETING), &[odd])
        .await;
    assert!(result.is_err());
}

#[test]
fn centroids_round_trip_bit_for_bit() {
    let values = vec![0.25, -1.5, f32::MIN_POSITIVE, 3.0e38];
    assert_eq!(voices::decode(&voices::encode(&values)), values);
    assert_eq!(voices::encode(&[1.0]), 1.0_f32.to_le_bytes().to_vec());
}
