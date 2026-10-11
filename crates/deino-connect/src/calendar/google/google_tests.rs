//! The Google connector against API answers shaped like Google's (fields
//! and values from the Calendar API v3 reference), through a scripted client.

use chrono::{DateTime, Utc};
use deino_core::calendar::{Availability, EventKind, EventStatus, ParticipantKind, SourceEvent};
use deino_domain::Rsvp;
use deino_net::{FakeNetClient, FakeReply};

use super::*;

const CALENDAR_LIST: &str = r##"{
  "kind": "calendar#calendarList",
  "items": [
    { "id": "maya@deino.dev", "summary": "maya@deino.dev", "backgroundColor": "#9fe1e7", "primary": true, "accessRole": "owner" },
    { "id": "en.usa#holiday@group.v.calendar.google.com", "summary": "Holidays in United States", "summaryOverride": "US holidays", "backgroundColor": "#16a765" }
  ]
}"##;

const WORK_EVENTS: &str = r#"{
  "kind": "calendar#events",
  "items": [
    {
      "id": "abc123_20261009T160000Z", "status": "confirmed", "summary": "Pricing review",
      "description": "Agenda in the doc", "iCalUID": "abc123@google.com", "recurringEventId": "abc123",
      "start": { "dateTime": "2026-10-09T09:00:00-07:00", "timeZone": "America/Los_Angeles" },
      "end": { "dateTime": "2026-10-09T09:30:00-07:00" },
      "hangoutLink": "https://meet.google.com/abc-defg-hij",
      "conferenceData": { "entryPoints": [ { "entryPointType": "video", "uri": "https://meet.google.com/abc-defg-hij" } ] },
      "organizer": { "email": "noor@deino.dev" },
      "attendees": [
        { "email": "maya@deino.dev", "self": true, "responseStatus": "accepted" },
        { "email": "noor@deino.dev", "organizer": true, "responseStatus": "accepted" },
        { "email": "c_188@resource.calendar.google.com", "displayName": "Blue room", "resource": true, "responseStatus": "accepted" },
        { "email": "marcus@brightkite.com", "displayName": "Marcus", "responseStatus": "needsAction" }
      ],
      "eventType": "default"
    },
    { "id": "ooo1", "summary": "Away", "eventType": "outOfOffice", "start": { "dateTime": "2026-10-10T00:00:00Z" }, "end": { "dateTime": "2026-10-11T00:00:00Z" } },
    { "id": "bday", "summary": "Ada's birthday", "eventType": "birthday", "start": { "date": "2026-10-12" }, "end": { "date": "2026-10-13" } },
    { "id": "free1", "summary": "Gym", "transparency": "transparent", "status": "cancelled", "start": { "dateTime": "2026-10-09T18:00:00Z" }, "end": { "dateTime": "2026-10-09T19:00:00Z" } }
  ]
}"#;

const HOLIDAY_EVENTS: &str = r#"{
  "items": [ { "id": "h1", "summary": "Columbus Day", "start": { "date": "2026-10-12" }, "end": { "date": "2026-10-13" } } ],
  "nextPageToken": "p2"
}"#;

const HOLIDAY_EVENTS_PAGE_2: &str = r#"{ "items": [] }"#;

fn window() -> SyncWindow {
    let at = |text: &str| {
        DateTime::parse_from_rfc3339(text)
            .unwrap()
            .with_timezone(&Utc)
    };
    SyncWindow {
        start: at("2026-09-09T00:00:00Z"),
        end: at("2026-10-23T00:00:00Z"),
    }
}

async fn snapshot() -> (SourceSnapshot, FakeNetClient) {
    let fake = FakeNetClient::default();
    for body in [
        CALENDAR_LIST,
        WORK_EVENTS,
        HOLIDAY_EVENTS,
        HOLIDAY_EVENTS_PAGE_2,
    ] {
        fake.reply(FakeReply::ok(body));
    }
    let snapshot = GoogleCalendar
        .snapshot(&fake, "access-1", window())
        .await
        .unwrap();
    (snapshot, fake)
}

fn work_event<'a>(snapshot: &'a SourceSnapshot, id: &str) -> &'a SourceEvent {
    let work = &snapshot.accounts[0].calendars[0];
    work.events
        .iter()
        .find(|event| event.provider_event_id == id)
        .unwrap()
}

#[tokio::test]
async fn the_account_is_named_by_its_primary_calendar_and_lists_every_calendar() {
    let (snapshot, _) = snapshot().await;

    let account = &snapshot.accounts[0];
    assert_eq!(account.name, "maya@deino.dev");
    let titles: Vec<&str> = account.calendars.iter().map(|c| c.title.as_str()).collect();
    assert_eq!(titles, ["maya@deino.dev", "US holidays"]);
    assert_eq!(
        account.calendars[0].color.as_ref().unwrap().as_str(),
        "#9FE1E7"
    );
}

#[tokio::test]
async fn a_meeting_keeps_its_meet_link_series_people_and_answers() {
    let (snapshot, _) = snapshot().await;

    let meeting = work_event(&snapshot, "abc123_20261009T160000Z");

    assert_eq!(meeting.start.to_rfc3339(), "2026-10-09T16:00:00+00:00");
    assert_eq!(
        meeting.conference_url.as_deref(),
        Some("https://meet.google.com/abc-defg-hij")
    );
    assert_eq!(meeting.series_key.as_deref(), Some("abc123@google.com"));
    assert_eq!(meeting.kind, Some(EventKind::Default));
    let me = meeting.attendees.iter().find(|p| p.is_self).unwrap();
    assert_eq!(me.rsvp, Some(Rsvp::Accepted));
    assert_eq!(meeting.attendees[2].kind, ParticipantKind::Room);
    assert_eq!(meeting.attendees[3].rsvp, Some(Rsvp::NeedsAction));
}

#[tokio::test]
async fn event_types_transparency_and_status_carry_over_and_birthdays_drop() {
    let (snapshot, _) = snapshot().await;

    assert_eq!(
        work_event(&snapshot, "ooo1").kind,
        Some(EventKind::OutOfOffice)
    );
    let gym = work_event(&snapshot, "free1");
    assert_eq!(
        (gym.availability, gym.status),
        (Availability::Free, EventStatus::Cancelled)
    );
    let work = &snapshot.accounts[0].calendars[0];
    assert!(
        work.events
            .iter()
            .all(|event| event.provider_event_id != "bday")
    );
    let holiday = &snapshot.accounts[0].calendars[1].events[0];
    assert!(holiday.all_day);
}

#[tokio::test]
async fn requests_carry_the_token_expand_recurrences_and_follow_pages() {
    let (_, fake) = snapshot().await;

    let requests = fake.requests();
    assert_eq!(requests.len(), 4);
    assert_eq!(
        requests[0].headers.get("authorization"),
        Some("Bearer access-1")
    );
    let holidays = &requests[2].url;
    assert!(
        holidays
            .path()
            .contains("/calendars/en.usa%23holiday@group.v.calendar.google.com/events")
    );
    let query: Vec<(String, String)> = holidays
        .query_pairs()
        .map(|(k, v)| (k.into(), v.into()))
        .collect();
    assert!(query.contains(&("singleEvents".into(), "true".into())));
    assert!(query.contains(&("timeMin".into(), "2026-09-09T00:00:00+00:00".into())));
    assert!(
        requests[3]
            .url
            .query_pairs()
            .any(|(k, v)| k == "pageToken" && v == "p2")
    );
}
