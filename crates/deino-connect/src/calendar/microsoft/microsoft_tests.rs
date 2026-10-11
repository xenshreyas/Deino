//! The Microsoft Graph connector against answers shaped like Graph's (fields
//! and values from the v1.0 reference), through a scripted client.

use chrono::{DateTime, Utc};
use deino_core::calendar::{Availability, EventKind, EventStatus, ParticipantKind, SourceEvent};
use deino_domain::Rsvp;
use deino_net::{FakeNetClient, FakeReply};

use super::*;

const ME: &str = r#"{ "displayName": "Maya Rao", "mail": "maya@contoso.com", "userPrincipalName": "maya@contoso.onmicrosoft.com" }"#;

const CALENDARS: &str = r##"{
  "value": [ { "id": "AAMk-cal-1", "name": "Calendar", "hexColor": "#e74c3c", "isDefaultCalendar": true } ],
  "@odata.nextLink": "https://graph.microsoft.com/v1.0/me/calendars?$skip=1"
}"##;

const CALENDARS_PAGE_2: &str =
    r#"{ "value": [ { "id": "AAMk-cal-2", "name": "Team", "hexColor": "" } ] }"#;

const EVENTS: &str = r#"{
  "value": [
    {
      "id": "AAMk-evt-1", "iCalUId": "040000008200E00074C5B7101A82E00807EA0A09", "subject": "Quarterly review",
      "body": { "contentType": "text", "content": "Join the call" },
      "start": { "dateTime": "2026-10-09T16:00:00.0000000", "timeZone": "UTC" },
      "end": { "dateTime": "2026-10-09T17:00:00.0000000", "timeZone": "UTC" },
      "isAllDay": false, "isCancelled": false, "isOrganizer": false, "showAs": "busy",
      "type": "occurrence", "seriesMasterId": "AAMk-master-1",
      "onlineMeeting": { "joinUrl": "https://teams.microsoft.com/l/meetup-join/19%3ameeting_x%40thread.v2/0" },
      "responseStatus": { "response": "tentativelyAccepted", "time": "2026-10-01T10:00:00Z" },
      "organizer": { "emailAddress": { "name": "Lee", "address": "lee@fabrikam.com" } },
      "attendees": [
        { "type": "required", "status": { "response": "accepted" }, "emailAddress": { "name": "Lee", "address": "lee@fabrikam.com" } },
        { "type": "resource", "status": { "response": "accepted" }, "emailAddress": { "name": "Room 4", "address": "room4@contoso.com" } }
      ]
    },
    {
      "id": "AAMk-evt-2", "subject": "Vacation", "showAs": "oof", "isAllDay": true,
      "start": { "dateTime": "2026-10-12T00:00:00.0000000", "timeZone": "UTC" },
      "end": { "dateTime": "2026-10-14T00:00:00.0000000", "timeZone": "UTC" }
    },
    {
      "id": "AAMk-evt-3", "subject": "Moved", "isCancelled": true, "isOrganizer": true, "showAs": "free",
      "start": { "dateTime": "2026-10-10T09:00:00.0000000", "timeZone": "UTC" },
      "end": { "dateTime": "2026-10-10T09:30:00.0000000", "timeZone": "UTC" },
      "organizer": { "emailAddress": { "name": "Maya", "address": "maya@contoso.com" } },
      "attendees": [ { "type": "required", "status": { "response": "none" }, "emailAddress": { "address": "MAYA@contoso.com" } } ]
    }
  ]
}"#;

const EMPTY_VIEW: &str = r#"{ "value": [] }"#;

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
    for body in [ME, CALENDARS, CALENDARS_PAGE_2, EVENTS, EMPTY_VIEW] {
        fake.reply(FakeReply::ok(body));
    }
    let snapshot = MicrosoftCalendar
        .snapshot(&fake, "access-1", window())
        .await
        .unwrap();
    (snapshot, fake)
}

fn event<'a>(snapshot: &'a SourceSnapshot, id: &str) -> &'a SourceEvent {
    let calendar = &snapshot.accounts[0].calendars[0];
    calendar
        .events
        .iter()
        .find(|event| event.provider_event_id == id)
        .unwrap()
}

#[tokio::test]
async fn the_account_is_the_signed_in_mailbox_with_every_calendar_page() {
    let (snapshot, _) = snapshot().await;

    let account = &snapshot.accounts[0];
    assert_eq!(account.name, "maya@contoso.com");
    let titles: Vec<&str> = account.calendars.iter().map(|c| c.title.as_str()).collect();
    assert_eq!(titles, ["Calendar", "Team"]);
    assert_eq!(
        account.calendars[0].color.as_ref().unwrap().as_str(),
        "#E74C3C"
    );
    assert_eq!(account.calendars[1].color, None);
}

#[tokio::test]
async fn an_occurrence_keeps_its_teams_link_series_and_the_users_answer() {
    let (snapshot, _) = snapshot().await;

    let review = event(&snapshot, "AAMk-evt-1");

    assert_eq!(review.start.to_rfc3339(), "2026-10-09T16:00:00+00:00");
    assert!(
        review
            .conference_url
            .as_deref()
            .unwrap()
            .starts_with("https://teams.microsoft.com/")
    );
    assert_eq!(review.series_key.as_deref(), Some("AAMk-master-1"));
    assert_eq!(review.notes.as_deref(), Some("Join the call"));
    let me = review.attendees.iter().find(|p| p.is_self).unwrap();
    assert_eq!(
        (me.email.as_deref(), me.rsvp),
        (Some("maya@contoso.com"), Some(Rsvp::Tentative))
    );
    assert_eq!(review.attendees[1].kind, ParticipantKind::Resource);
}

#[tokio::test]
async fn out_of_office_cancelled_and_own_entries_translate() {
    let (snapshot, _) = snapshot().await;

    let vacation = event(&snapshot, "AAMk-evt-2");
    assert_eq!(
        (vacation.kind, vacation.availability),
        (Some(EventKind::OutOfOffice), Availability::Unavailable)
    );
    assert!(vacation.all_day);
    let moved = event(&snapshot, "AAMk-evt-3");
    assert_eq!(moved.status, EventStatus::Cancelled);
    assert!(moved.organizer.as_ref().unwrap().is_self);
    assert_eq!(
        moved.attendees.len(),
        1,
        "the user's own entry is marked, not added twice"
    );
    assert!(moved.attendees[0].is_self);
}

#[tokio::test]
async fn requests_prefer_utc_text_and_follow_next_links() {
    let (_, fake) = snapshot().await;

    let requests = fake.requests();
    assert_eq!(requests.len(), 5);
    assert_eq!(
        requests[2].url.as_str(),
        "https://graph.microsoft.com/v1.0/me/calendars?$skip=1"
    );
    let view = &requests[3];
    assert!(
        view.url
            .path()
            .ends_with("/me/calendars/AAMk-cal-1/calendarView")
    );
    assert!(
        view.headers
            .get("prefer")
            .unwrap()
            .contains("outlook.timezone=\"UTC\"")
    );
    assert_eq!(view.headers.get("authorization"), Some("Bearer access-1"));
}
