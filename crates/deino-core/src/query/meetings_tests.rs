//! Library reads over the sample: listing and reading meetings, their
//! transcripts and action items.

use chrono::NaiveDate;
use deino_domain::{ActionItemStatus, MeetingId, MeetingStatus, Millis};

use super::*;
use crate::action_items::StatusFilter;
use crate::query::testing::{SALES, STANDUP, SUPPORT, library, ports};
use crate::query::{
    Assignee, EntityKind, ItemFilter, Library, QueryError, TranscriptQueries, TranscriptWindow,
};

fn day(text: &str) -> NaiveDate {
    text.parse().unwrap()
}

async fn ids(library: &Library, filter: MeetingFilter) -> Vec<String> {
    let meetings = library.meetings.list(&filter).await.unwrap();
    meetings
        .into_iter()
        .map(|meeting| meeting.id.to_string())
        .collect()
}

#[tokio::test]
async fn the_list_is_newest_first_and_honours_the_limit() {
    let library = library().await;

    let all = ids(&library, MeetingFilter::default()).await;
    let three = ids(
        &library,
        MeetingFilter {
            limit: 3,
            ..MeetingFilter::default()
        },
    )
    .await;

    assert_eq!(all.len(), 8);
    assert_eq!(three, [STANDUP, "meeting-fixture-one-on-one", SUPPORT]);
}

#[tokio::test]
async fn tags_and_folders_narrow_by_name_and_folders_include_their_subfolders() {
    let library = library().await;
    let by = |filter: MeetingFilter| ids(&library, filter);

    let tagged = by(MeetingFilter {
        tag: Some("customer".into()),
        ..MeetingFilter::default()
    })
    .await;
    let customers = by(MeetingFilter {
        folder: Some("Customers".into()),
        ..MeetingFilter::default()
    })
    .await;
    let brightkite = by(MeetingFilter {
        folder: Some("Brightkite".into()),
        ..MeetingFilter::default()
    })
    .await;

    assert_eq!(tagged, [SUPPORT, SALES]);
    assert_eq!(customers, [SUPPORT, SALES]);
    assert_eq!(brightkite, [SALES]);
}

#[tokio::test]
async fn people_companies_and_days_narrow_together() {
    let library = library().await;
    let by = |filter: MeetingFilter| ids(&library, filter);

    let with_owen = by(MeetingFilter {
        company: Some("fernhollow.co".into()),
        ..MeetingFilter::default()
    })
    .await;
    let maya_in_october = by(MeetingFilter {
        person: Some("maya@deino.dev".into()),
        from: Some(day("2026-10-01")),
        to: Some(day("2026-10-07")),
        ..MeetingFilter::default()
    })
    .await;

    assert_eq!(with_owen, [SUPPORT]);
    assert_eq!(
        maya_in_october,
        ["meeting-fixture-one-on-one", "meeting-fixture-interview"]
    );
}

#[tokio::test]
async fn bad_filters_are_errors_not_empty_lists() {
    let library = library().await;

    let backwards = library
        .meetings
        .list(&MeetingFilter {
            from: Some(day("2026-10-08")),
            to: Some(day("2026-10-01")),
            ..MeetingFilter::default()
        })
        .await;
    let unknown = library
        .meetings
        .list(&MeetingFilter {
            tag: Some("Nope".into()),
            ..MeetingFilter::default()
        })
        .await;

    assert!(matches!(backwards, Err(QueryError::InvalidRange)));
    assert!(matches!(
        unknown,
        Err(QueryError::NotFound {
            kind: EntityKind::Tag,
            ..
        })
    ));
}

#[tokio::test]
async fn a_summary_names_attendees_who_came_and_links_the_meeting() {
    let library = library().await;

    let meetings = library
        .meetings
        .list(&MeetingFilter {
            limit: 1,
            ..MeetingFilter::default()
        })
        .await
        .unwrap();
    let standup = &meetings[0];

    assert_eq!(standup.title, "Eng standup");
    assert_eq!(standup.status, MeetingStatus::Ready);
    assert_eq!(standup.duration_s, Some(434));
    assert_eq!(standup.held_at.to_rfc3339(), "2026-10-08T15:31:04+00:00");
    assert!(standup.attendees.contains(&"Maya Chen".to_owned()));
    assert!(
        !standup.attendees.contains(&"Noor Haddad".to_owned()),
        "Noor declined"
    );
    assert_eq!(standup.link, "deino://meeting/meeting-fixture-standup");
}

#[tokio::test]
async fn reading_a_meeting_brings_its_notes_items_and_files() {
    let library = library().await;

    let detail = library
        .meetings
        .read(&MeetingId::new(STANDUP))
        .await
        .unwrap();

    assert!(
        detail
            .people
            .iter()
            .any(|person| person.is_me && person.name == "Shreyas")
    );
    assert!(
        detail
            .ai_notes
            .as_deref()
            .unwrap()
            .starts_with("## Decisions")
    );
    assert_eq!(detail.my_notes, None);
    assert!(!detail.action_items.is_empty());
    assert!(detail.action_items.iter().all(|item| {
        item.link
            .as_deref()
            .unwrap()
            .starts_with("deino://meeting/meeting-fixture-standup?t=")
    }));
    assert!(detail.transcript_segments > 0);
}

#[tokio::test]
async fn reading_a_missing_meeting_is_not_found() {
    let library = library().await;

    let missing = library.meetings.read(&MeetingId::new("meeting-x")).await;

    assert!(matches!(
        missing,
        Err(QueryError::NotFound {
            kind: EntityKind::Meeting,
            ..
        })
    ));
}

/// The standup's transcript from `from` to `to`, at most `limit` lines.
async fn standup_page(
    transcripts: &TranscriptQueries,
    from: Option<u32>,
    to: Option<u32>,
    limit: u32,
) -> crate::query::TranscriptPage {
    let window = TranscriptWindow {
        from: from.map(Millis),
        to: to.map(Millis),
        limit,
    };
    transcripts
        .page(&MeetingId::new(STANDUP), window)
        .await
        .unwrap()
}

#[tokio::test]
async fn transcripts_page_by_time_with_a_cursor_and_named_speakers() {
    let transcripts = TranscriptQueries::new(ports().await);

    let first = standup_page(&transcripts, None, None, 2).await;
    let next = standup_page(&transcripts, first.next_from_ms, None, 1).await;
    let late = standup_page(&transcripts, Some(60_000), Some(90_000), 0).await;

    assert_eq!(first.lines.len(), 2);
    assert_eq!(first.lines[0].speaker, "Shreyas");
    assert!(first.lines[0].is_me);
    assert_eq!(next.lines[0].start_ms, first.next_from_ms.unwrap());
    assert!(
        late.lines
            .iter()
            .all(|line| (60_000..90_000).contains(&line.start_ms))
    );
    assert!(first.total_lines > 2);
}

#[tokio::test]
async fn a_backwards_window_is_invalid() {
    let transcripts = TranscriptQueries::new(ports().await);

    let window = TranscriptWindow {
        from: Some(Millis(10)),
        to: Some(Millis(10)),
        limit: 0,
    };
    let page = transcripts.page(&MeetingId::new(STANDUP), window).await;

    assert!(matches!(page, Err(QueryError::InvalidRange)));
}

#[tokio::test]
async fn action_items_filter_by_owner_and_status() {
    let library = library().await;
    let items = &library.action_items;
    let list = |filter: ItemFilter| async move { items.list(&filter).await.unwrap() };

    let mine = list(ItemFilter {
        assignee: Assignee::Me,
        ..ItemFilter::default()
    })
    .await;
    let done = list(ItemFilter {
        status: StatusFilter::Done,
        ..ItemFilter::default()
    })
    .await;
    let priya = list(ItemFilter {
        assignee: Assignee::Person("Priya".into()),
        ..ItemFilter::default()
    })
    .await;

    assert!(!mine.is_empty());
    assert!(
        mine.iter()
            .all(|item| item.assignee_is_me && item.status == ActionItemStatus::Open)
    );
    assert!(!done.is_empty());
    assert!(
        done.iter()
            .all(|item| item.status == ActionItemStatus::Done)
    );
    assert!(
        priya
            .iter()
            .all(|item| item.assignee.as_deref() == Some("Priya Nair"))
    );
}
