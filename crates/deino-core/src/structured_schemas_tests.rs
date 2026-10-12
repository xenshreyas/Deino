//! Every typed structured answer the app asks an LLM for reaches Anthropic as
//! a constrained structured output: no schema is recursive, and the rewrite
//! leaves nothing the API refuses. The answers whose types are private to
//! their feature are checked there (`templates` `Pick`, `redaction`
//! `Verdicts`, `insights` `answer_schema`).

use deino_llm::json_schema_for;
use deino_llm::providers::anthropic::schema::constrained_schema;
use schemars::JsonSchema;
use serde_json::Value;

fn assert_constrained<T: JsonSchema>() -> Value {
    let schema = json_schema_for::<T>();
    match constrained_schema(&schema) {
        Ok(sent) => sent,
        Err(problem) => panic!("{}: {problem}", T::schema_name()),
    }
}

#[test]
fn the_note_schema_is_a_structured_output() {
    let sent = assert_constrained::<crate::enhance::EnhancedDoc>();

    let block = &sent["properties"]["sections"]["items"]["properties"]["blocks"]["items"];
    assert_eq!(block["additionalProperties"], false);
    assert_eq!(
        block["properties"]["details"]["items"]["additionalProperties"],
        false
    );
}

#[test]
fn every_other_typed_answer_is_a_structured_output() {
    assert_constrained::<crate::notes::edit::EditAnswer>();
    assert_constrained::<crate::tags::classifier::TaggingAnswer>();
    assert_constrained::<deino_llm::extraction::ActionItemsAnswer>();
    assert_constrained::<deino_llm::brief::BriefAnswer>();
    assert_constrained::<deino_llm::followup::FollowUpDecision>();
    assert_constrained::<deino_llm::followup::FollowUpDraftAnswer>();
}
