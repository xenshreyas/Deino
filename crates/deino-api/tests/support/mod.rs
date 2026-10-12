//! The API over the seeded sample library, run in-process: the real
//! SQLite repositories and services, argon2 keys, UTC and a fixed "now".

#![allow(
    dead_code,
    reason = "each test binary uses a different part of the harness"
)]

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Method, Request, StatusCode, header};
use deino_api::{ApiServices, ApiState, LockProbe, WriteServices, router};
use deino_core::action_items::{
    ActionItemDeps, ActionItemService, NoActionItemEvents, OptionSources, SqliteActionItemRepo,
};
use deino_core::api_keys::{
    ApiKeyDeps, ApiKeyScope, ApiKeyService, Argon2Hasher, NewApiKey, OsTokens, SqliteApiKeyRepo,
};
use deino_core::companies::SqliteCompanyRepo;
use deino_core::enhance::SqliteEnhancedNotesRepo;
use deino_core::folders::SqliteFolderRepo;
use deino_core::meetings::{
    DiskMeetingFiles, MeetingService, NoMeetingWork, SqliteMeetingRepo, TrashDeps, TrashService,
};
use deino_core::notes::{
    MemoryNoteImages, NotePorts, NoteService, RecordedNotesEvents, SqliteNotesRepo,
};
use deino_core::people::SqlitePeopleRepo;
use deino_core::query::{Library, ReadContext, SAMPLE_NOW, seed_sample_library};
use deino_core::redaction::{NoRedaction, TextRedactor};
use deino_core::segments::SqliteSegmentRepo;
use deino_core::tags::{RecordedTagEvents, SqliteTagRepo, TagPorts, TagService};
use deino_core::transcript_import::{TranscriptImportPorts, TranscriptImporter};
use deino_db::Database;
use deino_domain::{Clock, FakeClock, FixedZone, LocalZone};
use deino_testkit::{at, test_db};
use serde_json::Value;
use tower::ServiceExt;

/// A lock the test flips.
#[derive(Default)]
pub struct TestLock(pub AtomicBool);

impl LockProbe for TestLock {
    fn is_locked(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

/// The API over a seeded library.
pub struct Harness {
    pub app: Router,
    pub services: Arc<ApiServices>,
    pub lock: Arc<TestLock>,
    pub database: Database,
}

/// A response's status and JSON body (`Null` when empty).
pub struct Reply {
    pub status: StatusCode,
    pub headers: axum::http::HeaderMap,
    pub body: Value,
}

impl Harness {
    /// The harness, with secrets redacted by `redactor`.
    pub async fn with_redactor(redactor: Arc<dyn TextRedactor>) -> Self {
        let database = test_db().await;
        let zone: Arc<dyn LocalZone> = Arc::new(FixedZone::hours(0).unwrap());
        seed_sample_library(&database, zone.as_ref()).await.unwrap();
        let clock: Arc<dyn Clock> = Arc::new(FakeClock::new(at(SAMPLE_NOW)));
        let context = ReadContext {
            zone: zone.clone(),
            clock: clock.clone(),
            embedder: None,
            redactor: redactor.clone(),
        };
        let lock = Arc::new(TestLock::default());
        let services = Arc::new(ApiServices {
            library: Arc::new(Library::sqlite(&database, &context)),
            redactor,
            zone: zone.clone(),
            keys: Arc::new(keys(&database, &clock)),
            lock: lock.clone(),
            writes: writes(&database, &zone, &clock),
        });
        let state = ApiState {
            services: services.clone(),
            port: None,
        };
        Self {
            app: router(state),
            services,
            lock,
            database,
        }
    }

    /// The harness without redaction.
    pub async fn new() -> Self {
        Self::with_redactor(Arc::new(NoRedaction)).await
    }

    /// A new key's token.
    pub async fn token(&self, scope: ApiKeyScope) -> String {
        let new = NewApiKey {
            name: "Test".into(),
            scope,
        };
        self.services.keys.create(&new).await.unwrap().token
    }

    /// A read-write key's token.
    pub async fn writer(&self) -> String {
        self.token(ApiKeyScope::ReadWrite).await
    }

    /// Sends `method uri` through the router, with `token` and a JSON `body`.
    pub async fn call(
        &self,
        (method, uri): (Method, &str),
        token: Option<&str>,
        body: Option<Value>,
    ) -> Reply {
        let mut request = Request::builder()
            .method(method)
            .uri(uri)
            .header(header::HOST, "127.0.0.1");
        if let Some(token) = token {
            request = request.header(header::AUTHORIZATION, format!("Bearer {token}"));
        }
        let body = match body {
            Some(json) => {
                request = request.header(header::CONTENT_TYPE, "application/json");
                Body::from(json.to_string())
            }
            None => Body::empty(),
        };
        let response = self
            .app
            .clone()
            .oneshot(request.body(body).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let headers = response.headers().clone();
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
        Reply {
            status,
            headers,
            body,
        }
    }

    /// `method uri` with `token` and a JSON `body`.
    pub async fn send(&self, (method, uri): (Method, &str), token: &str, body: Value) -> Reply {
        self.call((method, uri), Some(token), Some(body)).await
    }

    /// `DELETE uri` with `token`.
    pub async fn delete(&self, uri: &str, token: &str) -> Reply {
        self.call((Method::DELETE, uri), Some(token), None).await
    }

    /// `GET uri` with `token`.
    pub async fn get(&self, uri: &str, token: &str) -> Reply {
        self.call((Method::GET, uri), Some(token), None).await
    }
}

fn keys(database: &Database, clock: &Arc<dyn Clock>) -> ApiKeyService {
    ApiKeyService::new(ApiKeyDeps {
        repo: Arc::new(SqliteApiKeyRepo::new(database.clone())),
        hasher: Arc::new(Argon2Hasher),
        tokens: Arc::new(OsTokens),
        clock: clock.clone(),
    })
}

fn writes(database: &Database, zone: &Arc<dyn LocalZone>, clock: &Arc<dyn Clock>) -> WriteServices {
    let meetings = Arc::new(SqliteMeetingRepo::new(database.clone()));
    let tag_repo = Arc::new(SqliteTagRepo::new(database.clone()));
    let tags = Arc::new(TagService::new(TagPorts {
        tags: tag_repo.clone(),
        applied: tag_repo.clone(),
        events: Arc::new(RecordedTagEvents::default()),
        clock: clock.clone(),
    }));
    WriteServices {
        meetings: Arc::new(MeetingService::new(meetings.clone(), clock.clone())),
        trash: Arc::new(TrashService::new(TrashDeps {
            bin: meetings.clone(),
            files: Arc::new(DiskMeetingFiles::new("/none".into(), "/none".into())),
            clock: clock.clone(),
            work: Arc::new(NoMeetingWork),
        })),
        notes: Arc::new(NoteService::new(
            NotePorts {
                notes: Arc::new(SqliteNotesRepo::new(database.clone())),
                meetings,
                images: Arc::new(MemoryNoteImages::default()),
                events: Arc::new(RecordedNotesEvents::default()),
            },
            clock.clone(),
        )),
        action_items: Arc::new(action_items(database, (tag_repo, zone), clock)),
        importer: Arc::new(importer(database, &tags, clock)),
        tags,
    }
}

fn action_items(
    database: &Database,
    (tags, zone): (Arc<SqliteTagRepo>, &Arc<dyn LocalZone>),
    clock: &Arc<dyn Clock>,
) -> ActionItemService {
    let repo = Arc::new(SqliteActionItemRepo::new(database.clone()));
    let people = Arc::new(SqlitePeopleRepo::new(database.clone()));
    ActionItemService::new(ActionItemDeps {
        items: repo.clone(),
        batch: repo.clone(),
        reader: repo,
        me: people.clone(),
        options: OptionSources {
            people,
            companies: Arc::new(SqliteCompanyRepo::new(database.clone())),
            tags,
            folders: Arc::new(SqliteFolderRepo::new(database.clone())),
        },
        events: Arc::new(NoActionItemEvents),
        clock: clock.clone(),
        zone: zone.clone(),
    })
}

fn importer(
    database: &Database,
    tags: &Arc<TagService>,
    clock: &Arc<dyn Clock>,
) -> TranscriptImporter {
    let meetings = Arc::new(SqliteMeetingRepo::new(database.clone()));
    let segments = Arc::new(SqliteSegmentRepo::new(database.clone()));
    TranscriptImporter::new(TranscriptImportPorts {
        meetings: meetings.clone(),
        trash: meetings,
        speakers: segments.clone(),
        segments,
        notes: Arc::new(SqliteEnhancedNotesRepo::new(database.clone())),
        tags: tags.clone(),
        work: Arc::new(NoMeetingWork),
        clock: clock.clone(),
    })
}
