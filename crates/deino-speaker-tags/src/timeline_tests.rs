use super::*;

/// Zoom showing `name` as the speaker (`None` for a blank name).
fn seen(name: &str) -> Option<Sighting> {
    DisplayName::parse(name)
        .ok()
        .map(|name| (CallApp::Zoom, name))
}

/// Polls every 250 ms from `start`, one per entry.
fn poll(timeline: &mut Timeline, start: u32, sightings: &[Option<Sighting>]) {
    for (index, sighting) in sightings.iter().enumerate() {
        let at = start + 250 * u32::try_from(index).unwrap();
        timeline.observe(Millis(at), sighting.clone());
    }
}

fn spans(timeline: &mut Timeline) -> Vec<(u32, u32, String)> {
    timeline
        .take_closed()
        .into_iter()
        .map(|span| (span.start.0, span.end.0, span.name.to_string()))
        .collect()
}

#[test]
fn an_active_speaker_switch_splits_halfway_between_polls() {
    let mut timeline = Timeline::default();

    poll(
        &mut timeline,
        0,
        &[seen("Alex"), seen("Alex"), seen("Bea"), seen("Bea"), None],
    );

    assert_eq!(
        spans(&mut timeline),
        [(0, 375, "Alex".into()), (375, 875, "Bea".into())]
    );
}

#[test]
fn the_same_name_in_another_app_is_another_span() {
    let mut timeline = Timeline::default();

    timeline.observe(Millis(0), seen("Alex"));
    timeline.observe(
        Millis(250),
        Some((CallApp::Meet, DisplayName::parse("Alex").unwrap())),
    );
    timeline.interrupt();

    let closed = timeline.take_closed();
    assert_eq!(closed.len(), 2);
    assert_eq!(
        (closed[0].app, closed[1].app),
        (CallApp::Zoom, CallApp::Meet)
    );
}

#[test]
fn a_stall_closes_the_span_after_its_last_sighting() {
    let mut timeline = Timeline::default();

    poll(&mut timeline, 0, &[seen("Alex"), seen("Alex")]);
    timeline.observe(Millis(3_000), seen("Alex"));
    timeline.observe(Millis(3_250), None);

    assert_eq!(
        spans(&mut timeline),
        [(0, 375, "Alex".into()), (3_000, 3_125, "Alex".into())]
    );
}

#[test]
fn a_pause_closes_the_span_and_the_next_poll_starts_afresh() {
    let mut timeline = Timeline::default();

    poll(
        &mut timeline,
        0,
        &[seen("Alex"), seen("Alex"), seen("Alex")],
    );
    timeline.interrupt();
    poll(&mut timeline, 750, &[seen("Bea"), seen("Bea")]);
    timeline.interrupt();

    assert_eq!(
        spans(&mut timeline),
        [(0, 625, "Alex".into()), (750, 1_125, "Bea".into())]
    );
}

#[test]
fn silence_and_unknown_polls_leave_gaps() {
    let mut timeline = Timeline::default();

    poll(
        &mut timeline,
        0,
        &[None, seen("Alex"), None, None, seen("Alex"), None],
    );

    assert_eq!(
        spans(&mut timeline),
        [(125, 375, "Alex".into()), (875, 1_125, "Alex".into())]
    );
}

#[test]
fn closed_spans_are_taken_once() {
    let mut timeline = Timeline::default();
    poll(&mut timeline, 0, &[seen("Alex"), None]);

    assert_eq!(spans(&mut timeline).len(), 1);
    assert!(timeline.take_closed().is_empty());
}
