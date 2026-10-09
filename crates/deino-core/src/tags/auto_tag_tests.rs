//! [`AutoTagService`] with `MockProvider` on the `tagging` route: the
//! prompt it sends, the tags it applies and every skip rule (ORG-06).

use std::sync::Arc;

use async_trait::async_trait;
use deino_domain::{FakeClock, MeetingId, TagSource};
use deino_llm::{LlmError, MockProvider, OutputFormat, ProviderId, Route, mock_gateway};
use deino_testkit::at;

use super::*;
use crate::meetings::Meeting;

const NOW: &str = "2026-10-09T09:00:00Z";

/// The Settings switch, fixed.
struct Switch(bool);

#[async_trait]
impl AutoTagSwitch for Switch {
    async fn auto_tag_enabled(&self) -> bool {
        self.0
    }
}

struct Harness {
    auto: AutoTagService,
    tags: Arc<TagService>,
    repo: Arc<FakeTagRepo>,
    events: Arc<RecordedTagEvents>,
    mock: Arc<MockProvider>,
}

fn meeting(id: &str, title: &str, created_at: &str) -> Meeting {
    Meeting {
        id: MeetingId::new(id),
        title: title.into(),
        created_at: at(created_at),
        updated_at: at(created_at),
    }
}

fn harness(mock: MockProvider, enabled: bool) -> Harness {
    let repo = Arc::new(FakeTagRepo::with_meetings([
        meeting("fernhollow", "Fernhollow check-in", "2026-10-09T08:00:00Z"),
        meeting("northwind", "Northwind escalation", "2026-10-05T08:00:00Z"),
        meeting("empty", "Untitled", "2026-10-09T08:30:00Z"),
    ]));
    let events = Arc::new(RecordedTagEvents::default());
    let tags = Arc::new(TagService::new(TagPorts {
        tags: repo.clone(),
        applied: repo.clone(),
        events: events.clone(),
        clock: Arc::new(FakeClock::new(at(NOW))),
    }));
    let mock = Arc::new(mock);
    let auto = AutoTagService::new(AutoTagPorts {
        service: tags.clone(),
        tags: repo.clone(),
        applied: repo.clone(),
        digests: repo.clone(),
        switch: Arc::new(Switch(enabled)),
        gateway: Arc::new(mock_gateway(mock.clone()).gateway),
    });
    Harness {
        auto,
        tags,
        repo,
        events,
        mock,
    }
}

fn answer(names: &[&str]) -> MockProvider {
    let json = serde_json::json!({ "tags": names }).to_string();
    MockProvider::new().reply(Route::Tagging, &json)
}

fn id(meeting: &str) -> MeetingId {
    MeetingId::new(meeting)
}

impl Harness {
    async fn tag(&self, name: &str, description: &str) -> Tag {
        let new = NewTag {
            name: name.into(),
            color: None,
            description: description.into(),
        };
        self.tags.create(&new).await.unwrap()
    }

    /// `Customer` (on Northwind, by hand) and `Hiring`; Fernhollow has a summary.
    async fn seed(&self) -> (Tag, Tag) {
        let customer = self.tag("Customer", "Calls with paying customers.").await;
        let hiring = self.tag("Hiring", "Interviews and debriefs.").await;
        self.tags
            .apply(&id("northwind"), &customer.id, TagSource::User)
            .await
            .unwrap();
        self.repo
            .set_summary(&id("northwind"), "Duplicate jobs after v1.4.");
        self.repo
            .set_summary(&id("fernhollow"), "Fernhollow wants 15 more seats.");
        (customer, hiring)
    }
}

/// ORG-06 acceptance: the auto-tagger test with the mock provider passes.
#[tokio::test]
async fn existing_tags_the_model_picks_are_applied_as_ai_and_announced() {
    let harness = harness(answer(&["customer", "Board"]), true);
    let (customer, _) = harness.seed().await;

    let outcome = harness.auto.auto_tag(&id("fernhollow")).await.unwrap();

    assert_eq!(outcome, AutoTagged::Applied(vec![customer.id]));
    let applied = harness.tags.tags_of(&id("fernhollow")).await.unwrap();
    assert_eq!(applied.len(), 1);
    assert_eq!(
        (applied[0].tag.id, applied[0].applied_by),
        (customer.id, TagSource::Ai)
    );
    let last = harness.events.tagged().pop().unwrap();
    assert_eq!(
        (last.meeting_id, last.applied_by),
        (id("fernhollow"), TagSource::Ai)
    );
    assert!(
        harness
            .tags
            .list()
            .await
            .unwrap()
            .iter()
            .all(|tag| tag.name != "Board")
    );
}

#[tokio::test]
async fn the_prompt_goes_to_the_tagging_route_with_descriptions_examples_and_a_schema() {
    let harness = harness(answer(&[]), true);
    harness.seed().await;

    harness.auto.auto_tag(&id("fernhollow")).await.unwrap();

    let requests = harness.mock.requests();
    assert_eq!(requests.len(), 1);
    let request = &requests[0];
    assert_eq!(request.route, Route::Tagging);
    let OutputFormat::Json(output) = &request.output else {
        panic!("{:?}", request.output)
    };
    assert_eq!(output.name, "tagging_answer");
    let user = request.messages[0].text();
    assert!(
        user.contains("## Customer\nCalls with paying customers."),
        "{user}"
    );
    assert!(
        user.contains("- Northwind escalation: Duplicate jobs after v1.4."),
        "{user}"
    );
    assert!(
        user.contains("## Hiring\nInterviews and debriefs."),
        "{user}"
    );
    assert!(
        user.contains("Title: Fernhollow check-in\n\nSummary:\nFernhollow wants 15 more seats."),
        "{user}"
    );
}

/// ORG-06 acceptance: manual tags block auto-tagging.
#[tokio::test]
async fn a_meeting_the_user_tagged_is_skipped_without_calling_the_model() {
    let harness = harness(answer(&["Hiring"]), true);
    let (customer, _) = harness.seed().await;
    harness
        .tags
        .apply(&id("fernhollow"), &customer.id, TagSource::User)
        .await
        .unwrap();

    let outcome = harness.auto.auto_tag(&id("fernhollow")).await.unwrap();

    assert_eq!(outcome, AutoTagged::Skipped(AutoTagSkip::TaggedByUser));
    assert!(harness.mock.requests().is_empty());
    assert_eq!(
        harness.tags.tags_of(&id("fernhollow")).await.unwrap().len(),
        1
    );
}

#[tokio::test]
async fn tags_the_ai_applied_earlier_do_not_block_a_rerun() {
    let harness = harness(
        answer(&["Customer"]).reply(Route::Tagging, r#"{"tags":["Customer"]}"#),
        true,
    );
    let (customer, _) = harness.seed().await;

    harness.auto.auto_tag(&id("fernhollow")).await.unwrap();
    let rerun = harness.auto.auto_tag(&id("fernhollow")).await.unwrap();

    assert_eq!(rerun, AutoTagged::Applied(vec![customer.id]));
    assert_eq!(harness.mock.requests().len(), 2);
}

#[tokio::test]
async fn auto_tagging_is_skipped_when_settings_turn_it_off() {
    let harness = harness(answer(&["Customer"]), false);
    harness.seed().await;

    let outcome = harness.auto.auto_tag(&id("fernhollow")).await.unwrap();

    assert_eq!(outcome, AutoTagged::Skipped(AutoTagSkip::Disabled));
    assert!(harness.mock.requests().is_empty());
}

#[tokio::test]
async fn only_tags_with_auto_apply_on_are_offered() {
    let harness = harness(answer(&["Customer"]), true);
    let customer = harness.tag("Customer", "").await;
    let update = TagUpdate {
        name: customer.name.clone(),
        color: customer.color,
        description: String::new(),
        auto_apply: false,
    };
    harness.tags.update(&customer.id, &update).await.unwrap();
    harness.repo.set_summary(&id("fernhollow"), "Seats.");

    let outcome = harness.auto.auto_tag(&id("fernhollow")).await.unwrap();

    assert_eq!(outcome, AutoTagged::Skipped(AutoTagSkip::NoTags));
}

#[tokio::test]
async fn without_a_summary_the_transcript_opening_is_read_and_without_either_it_skips() {
    let harness = harness(answer(&[]), true);
    harness.seed().await;

    let empty = harness.auto.auto_tag(&id("empty")).await.unwrap();
    harness
        .repo
        .set_transcript(&id("empty"), "We saw duplicate jobs.");
    let from_transcript = harness.auto.auto_tag(&id("empty")).await.unwrap();
    let gone = harness.auto.auto_tag(&id("meeting-x")).await.unwrap();

    assert_eq!(empty, AutoTagged::Skipped(AutoTagSkip::NoContent));
    assert_eq!(from_transcript, AutoTagged::Applied(Vec::new()));
    assert_eq!(gone, AutoTagged::Skipped(AutoTagSkip::MeetingGone));
    let user = harness.mock.requests()[0].messages[0].text();
    assert!(
        user.contains("Transcript (opening):\nWe saw duplicate jobs."),
        "{user}"
    );
}

#[tokio::test]
async fn provider_failures_say_whether_a_retry_may_help() {
    let busy = LlmError::Overloaded {
        provider: ProviderId::BuiltIn,
        retry_after: None,
    };
    let no_key = LlmError::NoKey(ProviderId::BuiltIn);
    let script = MockProvider::new()
        .fail(Route::Tagging, busy)
        .fail(Route::Tagging, no_key);
    let harness = harness(script, true);
    harness.seed().await;

    let first = harness.auto.auto_tag(&id("fernhollow")).await.unwrap_err();
    let second = harness.auto.auto_tag(&id("fernhollow")).await.unwrap_err();

    assert!(first.is_retryable(), "{first:?}");
    assert!(!second.is_retryable(), "{second:?}");
    assert!(
        harness
            .tags
            .tags_of(&id("fernhollow"))
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn without_a_model_auto_tagging_is_skipped() {
    let tagged = NoModelAutoTagger.auto_tag(&id("fernhollow")).await.unwrap();

    assert_eq!(tagged, AutoTagged::Skipped(AutoTagSkip::NoModel));
}
