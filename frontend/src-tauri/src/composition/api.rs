//! Wiring for the local REST API (DEV-03): the keys service, and the
//! `ApiController` over the app's own services, so every API write takes
//! the single writer and reaches the UI through `DbChanged`. Once
//! everything else runs, the server starts if Settings → Developer has it
//! on.

use std::sync::Arc;

use deino_api::{ApiController, ApiServices, LockProbe, NeverLocked, WriteServices};
use deino_core::action_items::ActionItemService;
use deino_core::api_keys::{ApiKeyDeps, ApiKeyService, Argon2Hasher, OsTokens, SqliteApiKeyRepo};
use deino_core::enhance::SqliteEnhancedNotesRepo;
use deino_core::lock::LockService;
use deino_core::meetings::{MeetingService, SqliteMeetingRepo, TrashService};
use deino_core::notes::NoteService;
use deino_core::query::{Library, ReadContext};
use deino_core::redaction::{Redactor, SettingRedactor, TextRedactor};
use deino_core::segments::SqliteSegmentRepo;
use deino_core::settings::SettingsService;
use deino_core::tags::TagService;
use deino_core::transcript_import::{TranscriptImportPorts, TranscriptImporter};
use deino_db::Database;
use deino_domain::{Clock, LocalZone, SystemZone};
use tauri::{AppHandle, Manager};

use super::attach::{Context, Phase};
use super::meetings::PipelineWork;

super::attach::attach!("api", 90, Phase::Background(attach_api));

/// Manages `Arc<ApiKeyService>` and `Arc<ApiController>`, then starts the
/// server when its setting is on. Without the services it reads and writes
/// through (a failed startup), the API stays unavailable.
fn attach_api(cx: &Context<'_>) {
    let keys = Arc::new(api_keys(cx.database, cx.clock));
    cx.app.manage(Arc::clone(&keys));
    let Some(settings) = shared::<SettingsService>(cx.app) else {
        return log::error!(target: "api", "settings are not loaded; the local API is off");
    };
    let Some(writes) = writes(cx) else {
        return log::error!(target: "api", "a write service is missing; the local API is off");
    };
    let services = ApiServices {
        library: Arc::new(library(cx, &settings)),
        redactor: redactor(&settings),
        zone: Arc::new(SystemZone),
        keys,
        lock: lock(cx.app),
        writes,
    };
    let controller = Arc::new(ApiController::new(Arc::new(services), settings));
    cx.app.manage(Arc::clone(&controller));
    tauri::async_runtime::spawn(async move {
        controller.reconcile().await;
    });
}

/// A managed `Arc<T>`, when it is there.
fn shared<T: Send + Sync + 'static>(app: &AppHandle) -> Option<Arc<T>> {
    app.try_state::<Arc<T>>().map(|state| Arc::clone(&state))
}

fn api_keys(database: &Database, clock: &Arc<dyn Clock>) -> ApiKeyService {
    ApiKeyService::new(ApiKeyDeps {
        repo: Arc::new(SqliteApiKeyRepo::new(database.clone())),
        hasher: Arc::new(Argon2Hasher),
        tokens: Arc::new(OsTokens),
        clock: Arc::clone(clock),
    })
}

/// Exports' redaction: patterns alone, when "Hide sensitive information"
/// is on (NOT-09), as the CLI does.
fn redactor(settings: &Arc<SettingsService>) -> Arc<dyn TextRedactor> {
    Arc::new(SettingRedactor::new(
        settings.clone(),
        Arc::new(Redactor::regex_only()),
    ))
}

/// The CLI's read layer over the app's database. Search is by keyword: the
/// app's embedder is not shared yet (a follow-up).
fn library(cx: &Context<'_>, settings: &Arc<SettingsService>) -> Library {
    let zone: Arc<dyn LocalZone> = Arc::new(SystemZone);
    let context = ReadContext {
        zone,
        clock: Arc::clone(cx.clock),
        embedder: None,
        redactor: redactor(settings),
    };
    Library::sqlite(cx.database, &context)
}

fn lock(app: &AppHandle) -> Arc<dyn LockProbe> {
    match shared::<LockService>(app) {
        Some(lock) => lock,
        None => Arc::new(NeverLocked),
    }
}

/// The app's write services, and an importer over the same tags service.
fn writes(cx: &Context<'_>) -> Option<WriteServices> {
    let tags = shared::<TagService>(cx.app)?;
    let meetings = Arc::new(SqliteMeetingRepo::new(cx.database.clone()));
    let segments = Arc::new(SqliteSegmentRepo::new(cx.database.clone()));
    let importer = TranscriptImporter::new(TranscriptImportPorts {
        meetings: meetings.clone(),
        trash: meetings,
        speakers: segments.clone(),
        segments,
        notes: Arc::new(SqliteEnhancedNotesRepo::new(cx.database.clone())),
        tags: Arc::clone(&tags),
        work: Arc::new(PipelineWork(cx.app.clone())),
        clock: Arc::clone(cx.clock),
    });
    Some(WriteServices {
        meetings: shared::<MeetingService>(cx.app)?,
        trash: shared::<TrashService>(cx.app)?,
        notes: shared::<NoteService>(cx.app)?,
        action_items: shared::<ActionItemService>(cx.app)?,
        importer: Arc::new(importer),
        tags,
    })
}
