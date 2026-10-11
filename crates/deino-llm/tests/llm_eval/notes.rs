//! `notes.enhance` (NOT-02): the whole enhance engine over each fixture
//! meeting, seeded into SQLite, scored on what the user sees.
//!
//! - every line the user typed is in the notes, verbatim, as theirs;
//! - at least 90% of the AI's lines cite a segment of the meeting;
//! - the ground truth's decisions and facts are covered (a keyword check).
//!
//! `contract::notes.enhance::<case>` checks the prompt carries the user's
//! notes and the referenced transcript; `smoke::…` scores an oracle answer
//! built from the ground truth through `MockProvider`, so the scoring is
//! tested; `live::…` scores a real model (`DEINO_EVAL=1`).

mod oracle;
mod score;

use std::sync::Arc;

use async_trait::async_trait;
use deino_core::enhance::{
    EnhancePorts, EnhanceService, EnhancedNotesView, MaterialSources, Overwrite,
    SqliteEnhancedNotesRepo, TemplateSource,
};
use deino_core::templates::{
    BuiltinSync, CATALOG, GENERAL_TEMPLATE_ID, SqliteTemplateRepo, Template, TemplateError,
};
use deino_db::{Database, test_db};
use deino_domain::{MeetingId, SystemClock, TemplateId};
use deino_llm::{CompletionRequest, Gateway, Route, mock_gateway};
use deino_testkit::fixture::{FixtureMeeting, Fixtures, MeetingKind, seed_fixtures};
use libtest_mimic::{Failed, Trial};

use crate::live;

/// The eval's three trials for every fixture meeting.
pub fn trials(live_enabled: bool) -> Vec<Trial> {
    let Ok(fixtures) = Fixtures::load() else {
        return vec![Trial::test("setup::notes.enhance", || {
            Err("could not load the fixtures".into())
        })];
    };
    let rejects = fixtures.meetings().first().cloned();
    let mut trials: Vec<Trial> = fixtures
        .meetings()
        .iter()
        .flat_map(|meeting| meeting_trials(meeting, live_enabled))
        .collect();
    trials.push(Trial::test(
        format!(
            "smoke::{}::scoring-rejects-empty-notes",
            Route::NotesEnhance
        ),
        move || match rejects.map(|meeting| score::check(&meeting, &deino_doc::Doc::empty())) {
            Some(Err(_)) => Ok(()),
            _ => Err("empty notes passed the scoring".into()),
        },
    ));
    trials
}

fn meeting_trials(meeting: &FixtureMeeting, live_enabled: bool) -> Vec<Trial> {
    let name = format!(
        "{}::{}",
        Route::NotesEnhance,
        meeting.id.as_str().trim_start_matches("meeting-fixture-")
    );
    let (contract, smoke, live) = (meeting.clone(), meeting.clone(), meeting.clone());
    vec![
        Trial::test(format!("contract::{name}"), move || run_contract(&contract)),
        Trial::test(format!("smoke::{name}"), move || run_smoke(&smoke)),
        Trial::test(format!("live::{name}"), move || run_live(&live))
            .with_ignored_flag(!live_enabled),
    ]
}

/// The prompt carries every line the user typed and the referenced transcript.
fn run_contract(meeting: &FixtureMeeting) -> Result<(), Failed> {
    let mock = Arc::new(oracle::provider(meeting));
    enhance(meeting, mock_gateway(mock.clone()).gateway)?;
    let prompts: String = mock
        .requests()
        .iter()
        .filter(|request| request.route == Route::NotesEnhance)
        .map(prompt_text)
        .collect();
    let first = meeting
        .segments
        .first()
        .map_or("", |segment| segment.text.as_str());
    let mut needed: Vec<String> = score::typed_lines(meeting);
    needed.extend(["[s1 00:".to_owned(), first.trim().to_owned()]);
    match needed
        .iter()
        .find(|fragment| !prompts.contains(fragment.as_str()))
    {
        Some(missing) => Err(format!("the prompt lacks {missing:?}").into()),
        None => Ok(()),
    }
}

/// The oracle's answer through the engine passes the scoring.
fn run_smoke(meeting: &FixtureMeeting) -> Result<(), Failed> {
    let mock = Arc::new(oracle::provider(meeting));
    let view = enhance(meeting, mock_gateway(mock).gateway)?;
    score::check(meeting, &view.doc).map_err(Failed::from)
}

/// A real model's notes pass the scoring.
fn run_live(meeting: &FixtureMeeting) -> Result<(), Failed> {
    let view = enhance(meeting, live::gateway_for(Route::NotesEnhance)?)?;
    score::check(meeting, &view.doc).map_err(Failed::from)
}

/// Seeds the fixtures and writes `meeting`'s notes through `gateway`.
fn enhance(meeting: &FixtureMeeting, gateway: Gateway) -> Result<EnhancedNotesView, Failed> {
    let runtime = tokio::runtime::Runtime::new().map_err(|error| error.to_string())?;
    runtime.block_on(async {
        let database = seeded().await?;
        let service = service(&database, meeting, gateway);
        service
            .enhance(&meeting.id, Overwrite::KeepEdits)
            .await
            .map_err(|error| Failed::from(format!("enhance failed: {error:?}")))
    })
}

/// A database with every fixture meeting and the built-in templates.
async fn seeded() -> Result<Database, Failed> {
    let database = test_db().await;
    seed_fixtures(&database)
        .await
        .map_err(|error| error.to_string())?;
    let templates = Arc::new(SqliteTemplateRepo::new(database.clone()));
    BuiltinSync::new(templates, Arc::new(SystemClock))
        .run(&CATALOG)
        .await
        .map_err(|error| error.to_string())?;
    Ok(database)
}

fn service(database: &Database, meeting: &FixtureMeeting, gateway: Gateway) -> EnhanceService {
    let repo = Arc::new(SqliteEnhancedNotesRepo::new(database.clone()));
    let ports = EnhancePorts {
        sources: MaterialSources::sqlite(database),
        reader: repo.clone(),
        writer: repo,
        templates: Arc::new(KindTemplate(template_of(meeting.kind))),
        gateway: Arc::new(gateway),
    };
    EnhanceService::new(ports, Arc::new(SystemClock))
}

/// The built-in template auto selection picks for a meeting of `kind`.
fn template_of(kind: MeetingKind) -> Template {
    let id = match kind {
        MeetingKind::OneOnOne => "one_on_one",
        MeetingKind::Standup => "standup",
        MeetingKind::SalesDiscovery => "sales_discovery",
        MeetingKind::Interview => "candidate_interview",
        MeetingKind::BoardUpdate => "board_meeting",
        MeetingKind::CustomerSupport => "customer_support",
        MeetingKind::Brainstorm => "brainstorm",
        MeetingKind::Planning => GENERAL_TEMPLATE_ID,
    };
    let builtin = CATALOG.iter().find(|template| template.id == id);
    let content = builtin.map(deino_core::templates::BuiltinTemplate::content);
    Template {
        id: TemplateId::new(id),
        is_builtin: true,
        content: content.unwrap_or_default(),
        created_at: chrono::DateTime::UNIX_EPOCH,
        updated_at: chrono::DateTime::UNIX_EPOCH,
    }
}

/// Every meeting gets its kind's template.
struct KindTemplate(Template);

#[async_trait]
impl TemplateSource for KindTemplate {
    async fn template_for(&self, _meeting: &MeetingId) -> Result<Template, TemplateError> {
        Ok(self.0.clone())
    }
}

/// The system prompt and messages of a request, as one text.
fn prompt_text(request: &CompletionRequest) -> String {
    let messages: String = request
        .messages
        .iter()
        .map(deino_llm::Message::text)
        .collect();
    format!(
        "{}\n{messages}",
        request.system.as_deref().unwrap_or_default()
    )
}
