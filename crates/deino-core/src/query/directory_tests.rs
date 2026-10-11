//! The directory over the sample.

use chrono::NaiveDate;

use super::*;
use crate::query::testing::library;

#[tokio::test]
async fn people_match_by_name_or_email_and_carry_their_company() {
    let library = library().await;

    let everyone = library.directory.people(None, 0).await.unwrap();
    let brightkite = library
        .directory
        .people(Some("brightkite"), 0)
        .await
        .unwrap();
    let two = library.directory.people(None, 2).await.unwrap();

    assert!(
        everyone
            .iter()
            .any(|person| person.is_me && person.name == "Shreyas")
    );
    assert_eq!(brightkite.len(), 2);
    assert!(
        brightkite
            .iter()
            .all(|person| person.company.as_deref() == Some("Brightkite Logistics"))
    );
    assert_eq!(two.len(), 2);
}

#[tokio::test]
async fn companies_count_their_people() {
    let library = library().await;

    let companies = library.directory.companies().await.unwrap();
    let brightkite = companies
        .iter()
        .find(|company| company.name == "Brightkite Logistics")
        .unwrap();

    assert_eq!(brightkite.people, 2);
    assert_eq!(
        brightkite.domain.as_deref(),
        Some("brightkite-logistics.com")
    );
}

#[tokio::test]
async fn tags_count_meetings_and_folders_carry_their_path() {
    let library = library().await;

    let tags = library.directory.tags().await.unwrap();
    let folders = library.directory.folders().await.unwrap();

    let customer = tags.iter().find(|tag| tag.name == "Customer").unwrap();
    assert_eq!(customer.meetings, 2);
    let paths: Vec<&str> = folders.iter().map(|folder| folder.path.as_str()).collect();
    assert_eq!(paths, ["Customers", "Customers / Brightkite"]);
    assert_eq!(folders[1].meetings, 1);
}

#[tokio::test]
async fn events_default_to_a_fortnight_around_today_and_reject_long_windows() {
    let library = library().await;
    let day = |text: &str| text.parse::<NaiveDate>().unwrap();

    let around = library
        .directory
        .events(EventWindow::default())
        .await
        .unwrap();
    let too_long = library
        .directory
        .events(EventWindow {
            from: Some(day("2026-01-01")),
            to: Some(day("2026-12-31")),
        })
        .await;

    // The sample's "now" is 9 October: the window is 2 to 16 October.
    assert!(around.iter().any(|event| event.title == "Eng standup"));
    assert!(
        around
            .iter()
            .all(|event| event.start.date_naive() >= day("2026-10-02"))
    );
    assert!(matches!(too_long, Err(QueryError::InvalidRange)));
}
