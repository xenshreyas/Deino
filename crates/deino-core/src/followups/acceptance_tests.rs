//! NOT-07's acceptance checks over the seeded fixture library and the mock
//! LLM: a draft for the external sales call and none (without a model
//! call) for the internal standup; quiet verdicts, permanent dismissal,
//! kept edits, three days on Home, Mail and the clipboard, and redaction.

use std::sync::Arc;

use chrono::TimeDelta;
use deino_domain::{Email, MeetingId};
use deino_llm::followup::{FollowUpVerdict, placeholders};
use deino_llm::{MockProvider, Route};

use super::testing::{SALES, SECRET, STANDUP, WordRedactor, decision, seeded, sending, world};
use super::{
    FollowUpDrafter, FollowUpEdit, FollowUpError, FollowUpRun, Ineligible, Kept, QuietReason,
};

fn emails(draft_to: &[Email]) -> Vec<&str> {
    draft_to.iter().map(Email::as_str).collect()
}

fn edit(to: &[&str]) -> FollowUpEdit {
    FollowUpEdit {
        subject: "Pilot next steps".into(),
        body: "Hi Marcus,\nThe proposal follows.".into(),
        to: to.iter().map(|address| (*address).to_owned()).collect(),
        cc: vec![" ".into()],
        bcc: Vec::new(),
    }
}

#[tokio::test]
async fn the_external_sales_call_gets_a_draft_addressed_to_its_guests() {
    let database = seeded().await;
    let world = world(&database, sending(), None);
    let sales = MeetingId::new(SALES);

    let run = world.generator.draft(&sales).await.unwrap();

    assert_eq!(run, FollowUpRun::Drafted);
    let draft = world.service.for_meeting(&sales).await.unwrap().unwrap();
    assert_eq!(draft.subject, "Brightkite pilot: next steps");
    assert_eq!(placeholders(&draft.body), ["proposal date"]);
    assert_eq!(
        emails(&draft.recipients.to),
        [
            "hannah.pike@brightkite-logistics.com",
            "marcus.webb@brightkite-logistics.com"
        ]
    );
    let requests = world.llm.requests();
    assert_eq!(requests.len(), 2);
    let prompt = requests[0].messages[0].text();
    assert!(prompt.contains("- Marcus Webb, Brightkite Logistics (external)"));
    assert!(prompt.contains("- Shreyas, Deino (me)"));
    assert!(prompt.contains("- Lena Okafor, Deino\n"));
}

#[tokio::test]
async fn the_internal_standup_gets_nothing_and_asks_no_model() {
    let database = seeded().await;
    let world = world(&database, sending(), None);
    let standup = MeetingId::new(STANDUP);

    let run = world.generator.draft(&standup).await.unwrap();

    assert_eq!(run, FollowUpRun::Ineligible(Ineligible::Internal));
    assert!(world.llm.requests().is_empty());
    assert_eq!(world.service.for_meeting(&standup).await.unwrap(), None);
}

#[tokio::test]
async fn a_quiet_verdict_stores_no_draft_and_skips_the_second_call() {
    let database = seeded().await;
    let quiet = MockProvider::new().reply(
        Route::FollowupEmail,
        &decision(FollowUpVerdict::ColleagueOwns),
    );
    let world = world(&database, quiet, None);
    let sales = MeetingId::new(SALES);

    let run = world.generator.draft(&sales).await.unwrap();

    assert_eq!(run, FollowUpRun::Quiet(QuietReason::ColleagueOwns));
    assert_eq!(world.llm.requests().len(), 1);
    assert_eq!(world.service.for_meeting(&sales).await.unwrap(), None);
    assert!(world.service.suggested().await.unwrap().is_empty());
}

#[tokio::test]
async fn dismissing_is_permanent_even_when_the_pipeline_runs_again() {
    let database = seeded().await;
    let world = world(&database, sending(), None);
    let sales = MeetingId::new(SALES);
    world.generator.draft(&sales).await.unwrap();

    world.service.dismiss(&sales).await.unwrap();
    let rerun = world.generator.draft(&sales).await.unwrap();

    assert_eq!(rerun, FollowUpRun::Kept(Kept::Dismissed));
    assert_eq!(world.llm.requests().len(), 2);
    assert_eq!(world.service.for_meeting(&sales).await.unwrap(), None);
    assert!(world.service.suggested().await.unwrap().is_empty());
    let error = world.service.save(&sales, edit(&["a@b.example"])).await;
    assert!(matches!(error, Err(FollowUpError::NoDraft(_))));
}

#[tokio::test]
async fn an_edited_draft_survives_a_rerun_and_home_shows_it_for_three_days() {
    let database = seeded().await;
    let world = world(&database, sending(), None);
    let sales = MeetingId::new(SALES);
    world.generator.draft(&sales).await.unwrap();

    let saved = world
        .service
        .save(&sales, edit(&["Marcus.Webb@brightkite-logistics.com"]))
        .await
        .unwrap();
    let rerun = world.generator.draft(&sales).await.unwrap();

    assert_eq!(rerun, FollowUpRun::Kept(Kept::Edited));
    assert_eq!(saved.subject, "Pilot next steps");
    assert!(saved.recipients.cc.is_empty());
    let home = world.service.suggested().await.unwrap();
    assert_eq!(home.len(), 1);
    assert_eq!(
        home[0].meeting_title,
        "Deino × Brightkite Logistics: discovery"
    );
    world
        .clock
        .advance(TimeDelta::days(3) + TimeDelta::minutes(1));
    assert!(world.service.suggested().await.unwrap().is_empty());
    assert!(world.service.for_meeting(&sales).await.unwrap().is_some());
}

#[tokio::test]
async fn open_in_mail_saves_then_opens_an_encoded_draft_and_copy_takes_the_body() {
    let database = seeded().await;
    let world = world(&database, sending(), None);
    let sales = MeetingId::new(SALES);
    world.generator.draft(&sales).await.unwrap();

    world
        .service
        .open_in_mail(&sales, edit(&["marcus.webb@brightkite-logistics.com"]))
        .await
        .unwrap();
    world.service.copy(&sales, edit(&[])).await.unwrap();
    let invalid = world
        .service
        .open_in_mail(&sales, edit(&["not an address"]))
        .await;
    let nobody = world.service.open_in_mail(&sales, edit(&[])).await;

    assert_eq!(
        world.opener.urls(),
        [
            "mailto:marcus.webb%40brightkite-logistics.com?subject=Pilot%20next%20steps&body=Hi%20Marcus%2C%0D%0AThe%20proposal%20follows."
        ]
    );
    assert_eq!(
        world.clipboard.copies()[0].text,
        "Hi Marcus,\nThe proposal follows."
    );
    assert!(matches!(invalid, Err(FollowUpError::InvalidRecipient)));
    assert!(matches!(nobody, Err(FollowUpError::NoRecipients)));
}

#[tokio::test]
async fn the_notes_and_transcript_are_redacted_before_the_model_reads_them() {
    let database = seeded().await;
    let redactor = Arc::new(WordRedactor::default());
    let world = world(&database, sending(), Some(redactor.clone()));

    world.generator.draft(&MeetingId::new(SALES)).await.unwrap();

    let prompt = world.llm.requests()[0].messages[0].text().to_lowercase();
    assert!(!prompt.contains(SECRET), "the prompt leaks the secret");
    assert!(prompt.contains("[redacted]"));
    assert!(!redactor.seen.lock().unwrap().is_empty());
}
