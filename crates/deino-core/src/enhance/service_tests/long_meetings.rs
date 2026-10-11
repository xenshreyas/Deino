//! A long meeting is noted in parts (map) and merged (reduce), and the
//! transcript references survive the merge.

use super::*;

/// Every `s<n>` reference in `text`, in order, without repeats.
fn references(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    for word in text.split(|c: char| !c.is_alphanumeric()) {
        let is_reference = word.len() > 1
            && word.starts_with('s')
            && word[1..].chars().all(|c| c.is_ascii_digit());
        if is_reference && !found.iter().any(|seen: &String| seen == word) {
            found.push(word.to_owned());
        }
    }
    found
}

/// A part answers with its first line's reference; the merge cites the
/// first reference of every part it was given.
fn respond(request: &CompletionRequest) -> String {
    let prompt = request
        .messages
        .last()
        .map(deino_llm::Message::text)
        .unwrap_or_default();
    if prompt.contains("<partial_notes>") {
        let partials = prompt.split("<partial_notes>").nth(1).unwrap_or_default();
        let cited: Vec<String> = references(partials).into_iter().take(3).collect();
        let cited: Vec<&str> = cited.iter().map(String::as_str).collect();
        return answer_json(&[(
            "ai",
            "Q4 priorities are set and two projects are cut",
            &cited,
        )]);
    }
    let transcript = prompt.split("<transcript").nth(1).unwrap_or_default();
    let first = references(transcript)
        .into_iter()
        .next()
        .unwrap_or_default();
    answer_json(&[("ai", "This part's decisions", &[first.as_str()])])
}

#[tokio::test]
async fn an_eighty_minute_meeting_is_noted_in_parts_and_merged_with_its_references() {
    let harness = harness(
        MeetingKind::Planning,
        MockProvider::new().answer_with(Route::NotesEnhance, respond),
    )
    .await;
    let meeting = &harness.fixture.id;

    let view = harness
        .service
        .enhance(meeting, Overwrite::KeepEdits)
        .await
        .unwrap();

    let requests = enhance_requests(&harness.mock);
    let texts: Vec<String> = requests
        .iter()
        .filter_map(|r| r.messages.last().map(deino_llm::Message::text))
        .collect();
    let count = |needle: &str| texts.iter().filter(|text| text.contains(needle)).count();
    let (parts, merges) = (count("<transcript part="), count("<partial_notes>"));
    assert_eq!(
        (parts, merges),
        (4, 1),
        "20 + 20 + 20 + 23 minutes, then one merge"
    );
    assert!(
        requests.iter().all(|r| r.model == requests[0].model),
        "one model for every call"
    );
    let merge = texts
        .iter()
        .find(|text| text.contains("<partial_notes>"))
        .unwrap();
    assert!(
        merge.contains("[s1]"),
        "the first part's reference reaches the merge"
    );
    let cited = citations(&view.doc);
    assert_eq!(cited.len(), 3, "the merge's references resolve to segments");
    let ids = segment_ids(&harness.fixture);
    assert!(cited.iter().all(|id| ids.contains(&id.to_string())));
}

/// The IDs the seeder gives `fixture`'s segments.
fn segment_ids(fixture: &FixtureMeeting) -> Vec<String> {
    (0..fixture.segments.len())
        .map(|n| deino_testkit::fixture::fixture_uuid(&format!("segment/{}/{n}", fixture.id)))
        .collect()
}

#[test]
fn references_are_found_once_each() {
    assert_eq!(references("[s1 00:04] x [s12, s1] s3x"), ["s1", "s12"]);
}
