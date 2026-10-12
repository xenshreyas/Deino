//! Holdback scenarios: quiet call, echo dropped on a text match, double-talk
//! kept, late Them text, a lagging Them lane, the wall-time safety net.

use chrono::TimeZone;

use super::*;

fn t0() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, 11, 9, 0, 0).unwrap()
}

fn after(seconds: i64) -> DateTime<Utc> {
    t0() + TimeDelta::seconds(seconds)
}

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
fn a_me_segment_while_the_call_is_quiet_is_kept_once_them_has_caught_up() {
    let mut holdback = holdback();
    holdback.them_voice(span(0, 2_000));
    holdback.them_progress(Millis(9_000));

    let verdict = holdback.offer(
        offer(5_000, 7_000, "I'll take the action item", "me-1"),
        t0(),
    );

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
        t0(),
    );

    assert_eq!(verdict, Some(Verdict::Drop("echo")));
}

#[test]
fn a_held_segment_is_dropped_when_matching_them_text_arrives() {
    let mut holdback = holdback();
    holdback.them_voice(span(1_000, 4_000));
    assert_eq!(
        holdback.offer(
            offer(1_100, 3_900, "budget is approved for Q3", "echo"),
            t0()
        ),
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
fn double_talk_is_kept_once_the_them_lane_passes_its_end() {
    let mut holdback = holdback();
    holdback.them_voice(span(1_000, 4_000));
    holdback.them_final(
        Millis(1_000),
        Millis(4_000),
        "we should revisit pricing next quarter",
    );
    assert_eq!(
        holdback.offer(
            offer(2_000, 3_000, "sorry, can you repeat that", "me"),
            t0()
        ),
        None
    );

    assert!(holdback.them_progress(Millis(3_900)).is_empty());
    assert_eq!(holdback.them_progress(Millis(4_000)), [Verdict::Keep("me")]);
}

#[test]
fn a_lagging_them_lane_delays_the_verdict_instead_of_letting_echo_through() {
    let mut holdback = holdback();
    holdback.them_voice_started(Millis(1_000));
    assert_eq!(
        holdback.offer(offer(1_040, 3_040, "ship it on Friday", "echo"), t0()),
        None
    );

    // Seconds of wall time pass while the Them lane is still busy: nothing
    // is judged on time alone.
    assert!(holdback.tick(after(3)).is_empty());
    assert!(holdback.tick(after(10)).is_empty());
    assert!(holdback.them_progress(Millis(2_000)).is_empty());

    holdback.them_voice(span(1_000, 3_000));
    let verdicts = holdback.them_final(Millis(1_000), Millis(3_000), "let's ship it on Friday");
    assert_eq!(verdicts, [Verdict::Drop("echo")]);
}

#[test]
fn a_stalled_them_lane_releases_held_segments_after_the_wall_time_limit() {
    let mut holdback = holdback();
    holdback.them_voice(span(0, 5_000));
    holdback.offer(offer(1_000, 2_000, "anything at all", "me"), t0());

    assert!(holdback.tick(after(19)).is_empty());
    assert_eq!(holdback.tick(after(20)), [Verdict::Keep("me")]);
}

#[test]
fn words_them_repeats_later_do_not_make_a_quiet_line_echo() {
    let mut holdback = holdback();
    holdback.offer(offer(1_000, 2_000, "yes that sounds good", "me"), t0());
    holdback.them_voice(span(4_000, 5_000));

    let verdicts = holdback.them_final(Millis(4_000), Millis(5_000), "yes that sounds good");
    assert!(verdicts.is_empty(), "Them was quiet while Me spoke");
    assert_eq!(holdback.them_progress(Millis(5_000)), [Verdict::Keep("me")]);
}

#[test]
fn them_open_text_counts_as_evidence() {
    let mut holdback = holdback();
    holdback.them_live(Millis(10_000), "so the roadmap for the next two");
    let verdict = holdback.offer(
        offer(10_100, 12_000, "roadmap for the next two", "echo"),
        t0(),
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
        t0(),
    );
    assert_eq!(verdict, Some(Verdict::Drop("echo")));
}

#[test]
fn draining_judges_everything_still_held() {
    let mut holdback = holdback();
    holdback.them_voice(span(0, 5_000));
    holdback.offer(
        offer(500, 1_500, "totally different words here", "kept"),
        t0(),
    );
    holdback.offer(
        offer(2_000, 3_000, "quarterly numbers look strong", "echo"),
        t0(),
    );
    holdback.them_live(Millis(1_900), "our quarterly numbers look strong");

    assert_eq!(holdback.drain(), [Verdict::Keep("kept")]);
}
