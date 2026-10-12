use std::sync::Mutex;

use chrono::DateTime;
use deino_core::segments::WordTiming;
use deino_core::vocabulary::{Lexicon, RecordedLiveVocabulary, VocabTerm};
use deino_domain::{Channel, FakeClock, MeetingId, Millis, VocabKind, VocabTermId};

use super::*;
use crate::events::{RecordedEvents, SegmentUpdateKind};

/// Saves into a `Vec`, for assertions.
#[derive(Default)]
struct Saved(Mutex<Vec<NewSegment>>);

impl SegmentStore for Saved {
    fn save(&self, segments: &[NewSegment], _removed: &[SegmentId]) -> Result<(), StoreError> {
        self.0.lock().unwrap().extend_from_slice(segments);
        Ok(())
    }
}

fn shreyas() -> Arc<Corrector> {
    let term = VocabTerm {
        id: VocabTermId::generate(&FakeClock::new(DateTime::UNIX_EPOCH)),
        kind: VocabKind::Term,
        text: "Shreyas".into(),
        sounds_like: vec!["Sreyas".into()],
        created_at: DateTime::UNIX_EPOCH,
        updated_at: DateTime::UNIX_EPOCH,
    };
    Arc::new(Corrector::new(Lexicon::new(&[term], &[])))
}

fn row(id: SegmentId, text: &str) -> NewSegment {
    NewSegment {
        id,
        meeting_id: MeetingId::new("meeting-a"),
        channel: Channel::System,
        speaker_id: None,
        start: Millis(0),
        end: Millis(900),
        text: text.into(),
        words: vec![WordTiming {
            text: text.into(),
            start: Millis(0),
            end: Millis(900),
        }],
    }
}

/// A corrected store over [`Saved`], and the runtime its writes run on.
struct Fixture {
    store: CorrectedStore,
    saved: Arc<Saved>,
    _runtime: tokio::runtime::Runtime,
}

fn store(vocabulary: &Arc<RecordedLiveVocabulary>) -> Fixture {
    let saved = Arc::new(Saved::default());
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    let store = CorrectedStore::new(CorrectedStoreParts {
        inner: saved.clone(),
        corrector: shreyas(),
        vocabulary: vocabulary.clone(),
        runtime: runtime.handle().clone(),
    });
    Fixture {
        store,
        saved,
        _runtime: runtime,
    }
}

fn id() -> SegmentId {
    SegmentId::generate(&FakeClock::new(DateTime::UNIX_EPOCH))
}

#[test]
fn rows_are_saved_corrected_and_their_originals_kept() {
    let vocabulary = Arc::new(RecordedLiveVocabulary::default());
    let fixture = store(&vocabulary);
    let (sreyas, plain) = (id(), id());

    fixture
        .store
        .save(&[row(sreyas, "Sreyas"), row(plain, "hello")], &[])
        .unwrap();

    let texts: Vec<String> = fixture
        .saved
        .0
        .lock()
        .unwrap()
        .iter()
        .map(|r| r.text.clone())
        .collect();
    assert_eq!(texts, ["Shreyas", "hello"]);
    let kept = vocabulary.kept();
    assert_eq!(kept.len(), 1);
    assert_eq!(
        (kept[0].segment_id, kept[0].original_text.as_str()),
        (sreyas, "Sreyas")
    );
    assert_eq!(kept[0].corrected_words[0].text, "Shreyas");
}

#[test]
fn a_draft_no_longer_corrected_forgets_its_original_once() {
    let vocabulary = Arc::new(RecordedLiveVocabulary::default());
    let fixture = store(&vocabulary);
    let draft = id();
    fixture.store.save(&[row(draft, "Sreyas")], &[]).unwrap();

    fixture
        .store
        .save(&[row(draft, "Shreyas is here")], &[])
        .unwrap();
    fixture
        .store
        .save(&[row(draft, "Shreyas is here now")], &[])
        .unwrap();

    assert_eq!(vocabulary.forgotten(), [draft]);
}

#[test]
fn live_text_is_shown_corrected() {
    let recorded = Arc::new(RecordedEvents::default());
    let events = CorrectedEvents::new(recorded.clone(), shreyas());
    let update = SegmentUpdate {
        meeting_id: MeetingId::new("meeting-a"),
        segment_id: id(),
        channel: Channel::Mic,
        start: Millis(0),
        end: Millis(0),
        text: "thanks Sreyas".into(),
        state: SegmentUpdateKind::Open,
    };

    events.segment(&update);
    events.segment(&SegmentUpdate {
        text: "hello".into(),
        ..update
    });

    let texts: Vec<String> = recorded.segments().into_iter().map(|s| s.text).collect();
    assert_eq!(texts, ["thanks Shreyas", "hello"]);
}
