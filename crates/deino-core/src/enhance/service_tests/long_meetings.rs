//! A transcript too long for one call is noted in parts (map) and merged
//! (reduce), and the transcript references survive the merge.

use super::*;

/// A window too small for the 83-minute planning meeting's transcript
/// (about 76k characters) in one call, but roomy for a 23-minute part.
const SMALL_WINDOW: u32 = 20_000;

/// The planning meeting on a model with [`SMALL_WINDOW`], answered by `respond`.
async fn small_window_harness(
    respond: impl Fn(&CompletionRequest) -> String + Send + Sync + 'static,
) -> Harness {
    let mock = MockProvider::new()
        .with_context_window(SMALL_WINDOW)
        .answer_with(Route::NotesEnhance, respond);
    harness(MeetingKind::Planning, mock).await
}

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
async fn a_transcript_too_long_for_one_call_is_noted_in_parts_and_merged_with_its_references() {
    let harness = small_window_harness(respond).await;
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

/// Like [`respond`], but the merge and the second part's whole-part
/// answers never parse (the first part-2 prompt fixes how many lines the
/// whole part has): the part is noted in halves and merged here.
fn respond_with_a_broken_part() -> impl Fn(&CompletionRequest) -> String + Send + Sync {
    let whole = std::sync::Mutex::new(None::<usize>);
    move |request| {
        let prompt = request
            .messages
            .last()
            .map(deino_llm::Message::text)
            .unwrap_or_default();
        let lines = prompt.matches("[s").count();
        let broken = if prompt.contains("corrected JSON") || prompt.contains("<partial_notes>") {
            true
        } else if prompt.contains("part=\"2 of") {
            let mut whole = whole.lock().unwrap();
            *whole.get_or_insert(lines) == lines
        } else {
            false
        };
        if broken {
            "{\"sections\": [".to_owned()
        } else {
            respond(request)
        }
    }
}

#[tokio::test]
async fn a_part_whose_answer_cannot_be_read_still_leaves_notes() {
    let harness = small_window_harness(respond_with_a_broken_part()).await;

    let view = harness
        .service
        .enhance(&harness.fixture.id, Overwrite::KeepEdits)
        .await
        .unwrap();

    let part_two = enhance_requests(&harness.mock)
        .iter()
        .filter_map(|r| r.messages.last().map(deino_llm::Message::text))
        .filter(|text| text.contains("part=\"2 of"))
        .count();
    assert_eq!(
        part_two, 4,
        "asked twice (each repaired once), then two halves"
    );
    assert!(
        !text_by(&view.doc, Author::Ai).is_empty(),
        "the parts are merged without the model"
    );
    assert!(!citations(&view.doc).is_empty(), "their references survive");
}
