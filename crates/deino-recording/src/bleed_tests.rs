//! Holdback scenarios: quiet call, echo dropped on a text match, double-talk
//! kept, late Them text, expiry.

use super::*;

const T0: HostTime = HostTime(1_000_000_000);

fn holdback() -> Holdback<&'static str> {
    Holdback::new(HoldbackPolicy::default())
}

fn offer(start: u64, end: u64, text: &str, id: &'static str) -> Offer<&'static str> {
    Offer {
        start: Millis(start),
        end: Millis(end),
        text: text.into(),
        payload: id,
    }
}

fn span(start: u64, end: u64) -> VoiceSpan {
    VoiceSpan {
        start: Millis(start),
        end: Millis(end),
    }
}

#[test]
fn a_me_segment_while_the_call_is_quiet_is_kept_at_once() {
    let mut holdback = holdback();
    holdback.them_voice(span(0, 2_000));

    let verdict = holdback.offer(offer(5_000, 7_000, "I'll take the action item", "me-1"), T0);

    assert_eq!(verdict, Some(Verdict::Keep("me-1")));
}

#[test]
fn echo_of_text_them_already_said_is_dropped_at_once() {
    let mut holdback = holdback();
    holdback.them_final(
        Millis(1_000),
        Millis(4_000),
        "the launch moves to next Tuesday",
    );

    let verdict = holdback.offer(
        offer(1_050, 4_020, "launch moves to next Tuesday", "echo"),
        T0,
    );

    assert_eq!(verdict, Some(Verdict::Drop("echo")));
}

#[test]
fn a_held_segment_is_dropped_when_matching_them_text_arrives() {
    let mut holdback = holdback();
    holdback.them_voice(span(1_000, 4_000));
    assert_eq!(
        holdback.offer(offer(1_100, 3_900, "budget is approved for Q3", "echo"), T0),
        None
    );

    let verdicts = holdback.them_final(
        Millis(1_000),
        Millis(4_000),
        "the budget is approved for Q3",
    );

    assert_eq!(verdicts, [Verdict::Drop("echo")]);
    assert!(!holdback.is_holding());
}

#[test]
fn double_talk_with_different_words_is_kept_after_the_hold() {
    let mut holdback = holdback();
    holdback.them_voice(span(1_000, 4_000));
    holdback.them_final(
        Millis(1_000),
        Millis(4_000),
        "we should revisit pricing next quarter",
    );
    assert_eq!(
        holdback.offer(offer(2_000, 3_000, "sorry, can you repeat that", "me"), T0),
        None
    );

    assert!(holdback.tick(T0.plus(Duration::from_secs(2))).is_empty());
    let verdicts = holdback.tick(T0.plus(Duration::from_secs(3)));

    assert_eq!(verdicts, [Verdict::Keep("me")]);
}

#[test]
fn them_open_text_counts_as_evidence() {
    let mut holdback = holdback();
    holdback.them_live(Millis(10_000), "so the roadmap for the next two");
    let verdict = holdback.offer(
        offer(10_100, 12_000, "roadmap for the next two", "echo"),
        T0,
    );
    assert_eq!(verdict, Some(Verdict::Drop("echo")));
}

#[test]
fn an_echo_spanning_two_them_segments_matches_their_merge() {
    let mut holdback = holdback();
    holdback.them_final(Millis(0), Millis(2_000), "first we close the books");
    holdback.them_final(Millis(2_100), Millis(4_000), "then we file the report");
    let verdict = holdback.offer(
        offer(1_000, 3_500, "we close the books then we file", "echo"),
        T0,
    );
    assert_eq!(verdict, Some(Verdict::Drop("echo")));
}

#[test]
fn draining_judges_everything_still_held() {
    let mut holdback = holdback();
    holdback.them_voice(span(0, 5_000));
    holdback.offer(
        offer(500, 1_500, "totally different words here", "kept"),
        T0,
    );
    holdback.offer(
        offer(2_000, 3_000, "quarterly numbers look strong", "echo"),
        T0,
    );
    holdback.them_live(Millis(1_900), "our quarterly numbers look strong");

    assert_eq!(holdback.drain(), [Verdict::Keep("kept")]);
}
