//! A vocabulary over a fresh SQLite database, for the service and stage
//! tests: a meeting with a transcript, and people and companies inserted
//! with plain SQL (their own repositories' types evolve with other cards).

use std::sync::Arc;

use deino_db::{Database, test_db};
use deino_domain::{Channel, FakeClock, MeetingId, Millis, SegmentId, VocabKind};
use deino_testkit::{at, seed};

use super::{
    FixupPorts, FixupService, NameSources, SqliteCorrectionRepo, SqliteVocabRepo, TermInput,
    VocabTerm, VocabularyPorts, VocabularyService,
};
use crate::calendar::SqliteCalendarRepo;
use crate::companies::SqliteCompanyRepo;
use crate::meetings::SqliteMeetingRepo;
use crate::people::SqlitePeopleRepo;
use crate::segments::{
    NewSegment, Page, Segment, SegmentReader, SegmentWriter, SqliteSegmentRepo, WordTiming,
};

/// The meeting every world has.
pub(super) const MEETING: &str = "meeting-a";

/// A vocabulary, its stage and the database under them.
pub(super) struct World {
    pub database: Database,
    pub clock: Arc<FakeClock>,
    pub vocabulary: Arc<VocabularyService>,
    pub fixups: FixupService,
    pub segments: Arc<SqliteSegmentRepo>,
}

impl World {
    pub async fn new() -> Self {
        let database = test_db().await;
        seed::meeting(&database, MEETING, "Standup", at("2026-10-11T09:00:00Z")).await;
        let clock = Arc::new(FakeClock::new(at("2026-10-11T10:00:00Z")));
        let segments = Arc::new(SqliteSegmentRepo::new(database.clone()));
        let corrections = Arc::new(SqliteCorrectionRepo::new(database.clone()));
        let vocabulary = Arc::new(VocabularyService::new(VocabularyPorts {
            terms: Arc::new(SqliteVocabRepo::new(database.clone())),
            corrections: corrections.clone(),
            names: names(&database),
            clock: clock.clone(),
        }));
        let fixups = FixupService::new(FixupPorts {
            vocabulary: Arc::clone(&vocabulary),
            segments: segments.clone(),
            corrections,
            clock: clock.clone(),
        });
        Self {
            database,
            clock,
            vocabulary,
            fixups,
            segments,
        }
    }

    pub fn meeting() -> MeetingId {
        MeetingId::new(MEETING)
    }

    /// Adds a term or replacement.
    pub async fn add(&self, kind: VocabKind, text: &str, sounds_like: &[&str]) -> VocabTerm {
        let input = TermInput {
            kind,
            text: text.into(),
            sounds_like: sounds_like.iter().map(|&s| s.to_owned()).collect(),
        };
        self.vocabulary.create(&input).await.unwrap()
    }

    /// Adds a person named `name`.
    pub async fn person(&self, name: &str) {
        self.insert(
            "INSERT INTO people (id, name, created_at, updated_at) VALUES (?1, ?2, ?3, ?3)",
            name,
        )
        .await;
    }

    /// Adds a company named `name`.
    pub async fn company(&self, name: &str) {
        self.insert(
            "INSERT INTO companies (id, name, created_at, updated_at) VALUES (?1, ?2, ?3, ?3)",
            name,
        )
        .await;
    }

    async fn insert(&self, sql: &str, name: &str) {
        let id = SegmentId::generate(self.clock.as_ref()).to_string();
        sqlx::query(sql)
            .bind(id)
            .bind(name)
            .bind(at("2026-10-11T09:00:00Z"))
            .execute(self.database.writer())
            .await
            .unwrap();
    }

    /// Appends one segment per text, a second apart, each word timed.
    pub async fn transcript(&self, texts: &[&str]) -> Vec<NewSegment> {
        let rows: Vec<NewSegment> = (0_u32..)
            .zip(texts)
            .map(|(i, text)| self.segment(i * 1_000, text))
            .collect();
        self.segments.append(&rows).await.unwrap();
        rows
    }

    fn segment(&self, start: u32, text: &str) -> NewSegment {
        let words = (start..)
            .step_by(200)
            .zip(text.split_whitespace())
            .map(|(at, word)| WordTiming {
                text: word.into(),
                start: Millis(at),
                end: Millis(at + 200),
            })
            .collect();
        NewSegment {
            id: SegmentId::generate(self.clock.as_ref()),
            meeting_id: Self::meeting(),
            channel: Channel::System,
            speaker_id: None,
            start: Millis(start),
            end: Millis(start + 900),
            text: text.into(),
            words,
        }
    }

    /// The meeting's segments now.
    pub async fn segments(&self) -> Vec<Segment> {
        self.segments
            .list(&Self::meeting(), Page::ALL)
            .await
            .unwrap()
    }

    /// The meeting's texts now.
    pub async fn texts(&self) -> Vec<String> {
        self.segments().await.into_iter().map(|s| s.text).collect()
    }
}

fn names(database: &Database) -> NameSources {
    let people = Arc::new(SqlitePeopleRepo::new(database.clone()));
    NameSources {
        people,
        companies: Arc::new(SqliteCompanyRepo::new(database.clone())),
        meetings: Arc::new(SqliteMeetingRepo::new(database.clone())),
        calendar: Arc::new(SqliteCalendarRepo::new(database.clone())),
    }
}
