//! The retention tests' world: the real schema, the real trash and the real
//! file adapter over a temporary data folder, a simulated clock and the
//! periods set by the test.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::TimeDelta;
use deino_db::{Database, test_db};
use deino_domain::{FakeClock, MeetingId};
use deino_testkit::{at, seed};

use super::*;
use crate::meetings::trash::{DiskMeetingFiles, TrashDeps, TrashService};
use crate::meetings::{RecordedWork, SqliteMeetingRepo};
use crate::settings::{RetentionPeriod, RetentionSettings};

/// When every test meeting was saved.
pub(super) const SAVED: &str = "2026-10-01T09:00:00Z";

/// The periods, as the test sets them.
#[derive(Default)]
pub(super) struct FixedPolicy(Mutex<RetentionSettings>);

#[async_trait]
impl RetentionPolicy for FixedPolicy {
    async fn retention(&self) -> RetentionSettings {
        *self.0.lock().unwrap()
    }
}

pub(super) struct World {
    pub service: RetentionService,
    pub trash: Arc<TrashService>,
    pub database: Database,
    pub clock: Arc<FakeClock>,
    pub policy: Arc<FixedPolicy>,
    pub work: Arc<RecordedWork>,
    pub data: tempfile::TempDir,
}

/// A world on the day the meetings were saved, keeping everything.
pub(super) async fn world() -> World {
    let database = test_db().await;
    let data = tempfile::tempdir().unwrap();
    let disk = Arc::new(DiskMeetingFiles::new(
        data.path().join("recordings"),
        data.path().join("meetings"),
    ));
    let clock = Arc::new(FakeClock::new(at(SAVED)));
    let work = Arc::new(RecordedWork::default());
    let trash = Arc::new(TrashService::new(TrashDeps {
        bin: Arc::new(SqliteMeetingRepo::new(database.clone())),
        files: disk.clone(),
        clock: clock.clone(),
        work: work.clone(),
    }));
    let policy = Arc::new(FixedPolicy::default());
    let service = RetentionService::new(RetentionDeps {
        store: Arc::new(SqliteRetentionStore::new(database.clone())),
        policy: policy.clone(),
        files: disk,
        trash: trash.clone(),
        work: work.clone(),
        clock: clock.clone(),
    });
    World {
        service,
        trash,
        database,
        clock,
        policy,
        work,
        data,
    }
}

impl World {
    /// Keeps audio, transcripts and meetings for these periods.
    pub fn keep(&self, audio: RetentionPeriod, transcript: RetentionPeriod, all: RetentionPeriod) {
        *self.policy.0.lock().unwrap() = RetentionSettings {
            audio,
            transcript,
            everything: all,
        };
    }

    /// Moves the simulated clock `days` forward.
    pub fn days_pass(&self, days: i64) {
        self.clock.advance(TimeDelta::days(days));
    }

    /// `relative` inside the temporary data folder.
    pub fn path(&self, relative: &str) -> PathBuf {
        self.data.path().join(relative)
    }

    /// A finished meeting saved at [`SAVED`].
    pub async fn meeting(&self, id: &str) -> MeetingId {
        seed::meeting(&self.database, id, "Standup", at(SAVED)).await;
        MeetingId::new(id)
    }

    /// Runs `sql` with `binds`.
    pub async fn exec(&self, sql: &str, binds: &[&str]) {
        let mut query = sqlx::query(sql);
        for bind in binds {
            query = query.bind(*bind);
        }
        query.execute(self.database.writer()).await.unwrap();
    }

    /// The single number `sql` returns.
    pub async fn count(&self, sql: &str) -> i64 {
        sqlx::query_scalar(sql)
            .fetch_one(self.database.reader())
            .await
            .unwrap()
    }

    /// Records `path` (created on disk) as the meeting's microphone track.
    pub async fn record(&self, meeting: &str, path: &Path) {
        touch(path);
        let path = path.display().to_string();
        self.exec(
            "UPDATE meetings SET audio_mic_path = ?2 WHERE id = ?1",
            &[meeting, &path],
        )
        .await;
    }

    /// A clip `n` of the meeting, its file at `path` (created on disk).
    pub async fn clip(&self, meeting: &str, n: u32, path: &Path) {
        touch(path);
        let id = format!("00000000-0000-7000-8000-{n:012}");
        let path = path.display().to_string();
        self.exec(
            "INSERT INTO clips (id, meeting_id, name, start_ms, end_ms, media, path, created_at, updated_at)
             VALUES (?1, ?2, 'Clip', 0, 1000, 'audio', ?3, ?4, ?4)",
            &[&id, meeting, &path, SAVED],
        )
        .await;
    }

    /// A transcript line `n` saying `text`, and an embedding of it.
    pub async fn line(&self, meeting: &str, n: u32, text: &str) {
        let id = format!("00000000-0000-7000-9000-{n:012}");
        self.exec(
            "INSERT INTO transcript_segments (id, meeting_id, channel, start_ms, end_ms, text)
             VALUES (?1, ?2, 'mic', 0, 1000, ?3)",
            &[&id, meeting, text],
        )
        .await;
        self.embed(meeting, "transcript", &id, text).await;
    }

    /// An embedding of `text` from `kind` (`transcript`, `note`, `summary`).
    pub async fn embed(&self, meeting: &str, kind: &str, reference: &str, text: &str) {
        self.exec(
            "INSERT INTO embeddings (meeting_id, ref_kind, ref_id, chunk_ix, text, model, vector, created_at)
             VALUES (?1, ?2, ?3, (SELECT count(*) FROM embeddings WHERE meeting_id = ?1 AND ref_kind = ?2),
                     ?4, 'test', x'0000803f', ?5)",
            &[meeting, kind, reference, text, SAVED],
        )
        .await;
    }
}

/// Creates `path` and its folders, with a few bytes in it.
pub(super) fn touch(path: &Path) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, b"audio").unwrap();
}
