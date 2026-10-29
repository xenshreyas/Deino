use chrono::DateTime;
use deino_domain::{FakeClock, MeetingId, Millis, PaletteColor};

use super::*;

fn clock() -> FakeClock {
    FakeClock::new(DateTime::UNIX_EPOCH)
}

fn name(text: &str) -> DisplayName {
    DisplayName::parse(text).unwrap()
}

fn speaker(label: &str) -> Speaker {
    Speaker {
        id: SpeakerId::generate(&clock()),
        meeting_id: MeetingId::new("meeting"),
        label: label.into(),
        person_id: None,
        is_me: false,
        color: PaletteColor::cycle(0),
    }
}

/// A meeting's system track: segments, and which name (if any) won each.
#[derive(Default)]
struct Track {
    segments: Vec<Seated>,
    wins: Vec<Win>,
}

impl Track {
    /// Adds a segment of `length` on `speaker`, won by `winner`.
    fn say(&mut self, speaker: &Speaker, length: u32, winner: Option<&str>) -> SegmentId {
        let id = SegmentId::generate(&clock());
        self.segments.push(Seated {
            id,
            speaker: Some(speaker.id),
            length,
        });
        if let Some(winner) = winner {
            self.wins.push(Win {
                segment: id,
                name: name(winner),
                covered: Millis(length),
            });
        }
        id
    }

    fn elect(&self, speakers: &[Speaker], people: &[(DisplayName, PersonId)]) -> Election {
        elect(&Ballot {
            wins: &self.wins,
            segments: &self.segments,
            speakers,
            people,
        })
    }
}

fn seat(election: &Election, who: &str) -> Seat {
    election.seat_of(&name(who)).unwrap()
}

fn moved(election: &Election) -> Vec<(SegmentId, String)> {
    election
        .moves
        .iter()
        .map(|change| (change.segment, change.name.to_string()))
        .collect()
}

#[test]
fn without_diarization_the_main_name_takes_them_and_others_get_new_speakers() {
    let them = speaker("Them");
    let mut track = Track::default();
    track.say(&them, 4_000, Some("Alex"));
    track.say(&them, 3_000, Some("Alex"));
    let bea = track.say(&them, 2_000, Some("Bea"));
    track.say(&them, 1_000, None);

    let election = track.elect(std::slice::from_ref(&them), &[]);

    assert_eq!(seat(&election, "Alex"), Seat::Speaker(them.id));
    assert!(election.elected[0].takes_name);
    assert_eq!(seat(&election, "Bea"), Seat::New);
    assert_eq!(moved(&election), [(bea, "Bea".into())]);
}

#[test]
fn diarized_voices_take_their_majority_names_and_stray_segments_move() {
    let (one, two) = (speaker("Speaker 1"), speaker("Speaker 2"));
    let mut track = Track::default();
    track.say(&one, 5_000, Some("Alex"));
    let stray = track.say(&one, 1_000, Some("Bea"));
    track.say(&two, 4_000, Some("Bea"));
    track.say(&two, 500, None);

    let election = track.elect(&[one.clone(), two.clone()], &[]);

    assert_eq!(seat(&election, "Alex"), Seat::Speaker(one.id));
    assert_eq!(seat(&election, "Bea"), Seat::Speaker(two.id));
    assert_eq!(moved(&election), [(stray, "Bea".into())]);
}

#[test]
fn a_known_persons_own_speaker_is_their_seat() {
    let person = PersonId::generate(&clock());
    let named = Speaker {
        person_id: Some(person),
        ..speaker("Speaker 1")
    };
    let open = speaker("Speaker 2");
    let mut track = Track::default();
    track.say(&named, 3_000, Some("Alex Chen"));
    let joined = track.say(&open, 2_000, Some("Alex Chen"));

    let election = track.elect(&[named.clone(), open], &[(name("Alex Chen"), person)]);

    assert_eq!(seat(&election, "Alex Chen"), Seat::Speaker(named.id));
    assert!(!election.elected[0].takes_name);
    assert_eq!(moved(&election), [(joined, "Alex Chen".into())]);
}

#[test]
fn segments_of_a_speaker_the_user_named_or_of_the_user_never_move() {
    let named = Speaker {
        person_id: Some(PersonId::generate(&clock())),
        ..speaker("Speaker 1")
    };
    let me = Speaker {
        is_me: true,
        ..speaker("Me")
    };
    let mut track = Track::default();
    track.say(&named, 3_000, Some("Bea"));
    track.say(&me, 3_000, Some("Bea"));

    let election = track.elect(&[named, me], &[]);

    assert_eq!(seat(&election, "Bea"), Seat::New);
    assert!(election.moves.is_empty());
}

#[test]
fn exactly_half_of_a_voice_is_not_a_majority() {
    let them = speaker("Them");
    let mut track = Track::default();
    let alex = track.say(&them, 2_000, Some("Alex"));
    track.say(&them, 2_000, None);

    let election = track.elect(&[them], &[]);

    assert_eq!(seat(&election, "Alex"), Seat::New);
    assert_eq!(moved(&election), [(alex, "Alex".into())]);
}

#[test]
fn a_label_from_an_earlier_run_keeps_its_name_and_nothing_moves_again() {
    let labelled = speaker("Alex");
    let mut track = Track::default();
    track.say(&labelled, 3_000, Some("Alex"));

    let election = track.elect(std::slice::from_ref(&labelled), &[]);

    assert_eq!(seat(&election, "Alex"), Seat::Speaker(labelled.id));
    assert!(!election.elected[0].takes_name);
    assert!(election.moves.is_empty());
}

#[test]
fn a_labelled_speaker_takes_the_person_once_one_matches() {
    let labelled = speaker("Alex");
    let person = PersonId::generate(&clock());
    let mut track = Track::default();
    track.say(&labelled, 3_000, Some("Alex"));

    let election = track.elect(std::slice::from_ref(&labelled), &[(name("Alex"), person)]);

    assert_eq!(seat(&election, "Alex"), Seat::Speaker(labelled.id));
    assert!(election.elected[0].takes_name);
    assert_eq!(election.elected[0].person, Some(person));
}

#[test]
fn one_voice_goes_to_one_name_the_one_with_more_speech() {
    let them = speaker("Them");
    let other = speaker("Speaker 2");
    let mut track = Track::default();
    track.say(&them, 6_000, Some("Alex"));
    track.say(&other, 1_000, Some("Bea"));
    track.say(&other, 1_500, Some("Alex"));

    let election = track.elect(&[them.clone(), other.clone()], &[]);

    assert_eq!(election.elected[0].name, name("Alex"));
    assert_eq!(seat(&election, "Alex"), Seat::Speaker(them.id));
    assert_eq!(seat(&election, "Bea"), Seat::New);
}

#[test]
fn a_name_seen_for_part_of_the_call_claims_the_one_voice_it_was_always_on() {
    let mehrdad_voice = speaker("Speaker 2");
    let jordon_voice = speaker("Speaker 1");
    let mut track = Track::default();
    for _ in 0..4 {
        track.say(&mehrdad_voice, 30_000, Some("Mehrdad"));
    }
    for _ in 0..10 {
        track.say(&mehrdad_voice, 60_000, None);
        track.say(&jordon_voice, 60_000, None);
    }
    let early = track.say(&jordon_voice, 5_000, Some("Mehrdad"));

    let election = track.elect(&[mehrdad_voice.clone(), jordon_voice], &[]);

    assert_eq!(
        seat(&election, "Mehrdad"),
        Seat::Speaker(mehrdad_voice.id),
        "120 s of 125 s won are on one voice"
    );
    assert_eq!(
        moved(&election),
        [(early, "Mehrdad".to_owned())],
        "only the stray line moves"
    );
}

#[test]
fn a_contested_or_brief_name_gets_its_own_speaker() {
    let voice = speaker("Speaker 1");
    let mut track = Track::default();
    track.say(&voice, 50_000, Some("Mehrdad"));
    track.say(&voice, 600_000, None);
    assert_eq!(
        seat(&track.elect(std::slice::from_ref(&voice), &[]), "Mehrdad"),
        Seat::New,
        "under a minute"
    );

    track.say(&voice, 30_000, Some("Mehrdad"));
    track.say(&voice, 10_000, Some("Jordon"));
    assert_eq!(
        seat(&track.elect(&[voice], &[]), "Mehrdad"),
        Seat::New,
        "another name was seen on it"
    );
}
