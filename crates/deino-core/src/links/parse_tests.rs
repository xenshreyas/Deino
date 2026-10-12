//! `DeepLink::parse`: every route, its aliases and what each refuses.

use std::str::FromStr;

use deino_domain::{CalendarEventId, MeetingId, Millis};

use super::{DeepLink, LinkError, MAX_LINK_CHARS};

const EVENT: &str = "0190f5a2-7c1e-7b3a-9d2e-5f6a7b8c9d0e";

fn parse(text: &str) -> DeepLink {
    DeepLink::parse(text).unwrap_or_else(|error| panic!("{text}: {error}"))
}

fn refused(text: &str) -> LinkError {
    DeepLink::parse(text).expect_err(text)
}

fn meeting(id: &str, at: Option<u32>) -> DeepLink {
    DeepLink::Meeting {
        id: MeetingId::new(id),
        at: at.map(Millis),
    }
}

#[test]
fn a_meeting_link_opens_the_meeting_at_an_optional_moment() {
    let id = "meeting-fixture-standup";
    assert_eq!(parse(&format!("deino://meeting/{id}")), meeting(id, None));
    assert_eq!(
        parse(&format!("deino://meeting/{id}?t=231259")),
        meeting(id, Some(231_259))
    );
    assert_eq!(parse(&format!("deino://meeting/{id}/")), meeting(id, None));
    assert_eq!(parse(&format!("DEINO://Meeting/{id}")), meeting(id, None));
    assert_eq!(parse(&format!("deino:///meeting/{id}")), meeting(id, None));
}

#[test]
fn the_exports_plural_meetings_link_is_accepted_as_an_alias() {
    let id = "meeting-0b5f2c";
    assert_eq!(parse(&format!("deino://meetings/{id}")), meeting(id, None));
    assert_eq!(
        parse(&format!("deino://meetings/{id}?t=5")),
        meeting(id, Some(5))
    );
}

#[test]
fn a_meeting_link_refuses_a_bad_id_or_moment() {
    assert!(matches!(
        refused("deino://meeting/a%20b"),
        LinkError::Invalid(_)
    ));
    let long = "m".repeat(129);
    assert!(matches!(
        refused(&format!("deino://meeting/{long}")),
        LinkError::Invalid(_)
    ));
    assert!(matches!(
        refused("deino://meeting/abc?t=soon"),
        LinkError::Invalid(_)
    ));
    assert!(matches!(
        refused("deino://meeting/abc?t=-4"),
        LinkError::Invalid(_)
    ));
    assert!(matches!(
        refused("deino://meeting"),
        LinkError::UnknownRoute
    ));
    assert!(matches!(
        refused("deino://meeting/abc/extra"),
        LinkError::UnknownRoute
    ));
}

#[test]
fn a_new_note_link_takes_an_optional_calendar_event() {
    assert_eq!(parse("deino://new-note"), DeepLink::NewNote { event: None });
    let event = CalendarEventId::from_str(EVENT).unwrap();
    assert_eq!(
        parse(&format!("deino://new-note?event={EVENT}")),
        DeepLink::NewNote { event: Some(event) }
    );
    assert!(matches!(
        refused("deino://new-note?event=tomorrow"),
        LinkError::Invalid(_)
    ));
}

#[test]
fn record_links_start_and_stop() {
    assert_eq!(parse("deino://record/start"), DeepLink::RecordStart);
    assert_eq!(parse("deino://record/stop"), DeepLink::RecordStop);
    assert_eq!(parse("deino://Record/Stop/"), DeepLink::RecordStop);
    assert!(matches!(
        refused("deino://record/pause"),
        LinkError::UnknownRoute
    ));
    assert!(matches!(refused("deino://record"), LinkError::UnknownRoute));
}

#[test]
fn a_search_link_needs_a_query_and_decodes_it() {
    assert_eq!(
        parse("deino://search?q=release%20cut"),
        DeepLink::Search {
            query: "release cut".to_owned()
        }
    );
    assert_eq!(
        parse("deino://search?q=+Q4+plan+&x=1"),
        DeepLink::Search {
            query: "Q4 plan".to_owned()
        }
    );
    assert!(matches!(refused("deino://search"), LinkError::Missing(_)));
    assert!(matches!(
        refused("deino://search?q=%20%20"),
        LinkError::Missing(_)
    ));
    let long = "a".repeat(1_001);
    assert!(matches!(
        refused(&format!("deino://search?q={long}")),
        LinkError::Invalid(_)
    ));
}

#[test]
fn a_recipe_link_needs_its_data() {
    assert_eq!(
        parse("deino://recipe?data=eyJuYW1lIjoiMToxIn0"),
        DeepLink::Recipe {
            data: "eyJuYW1lIjoiMToxIn0".to_owned()
        }
    );
    assert!(matches!(refused("deino://recipe"), LinkError::Missing(_)));
    assert!(matches!(
        refused("deino://recipe?data=a%0Ab"),
        LinkError::Invalid(_)
    ));
}

#[test]
fn other_schemes_routes_and_oversized_links_are_refused() {
    assert!(matches!(
        refused("https://deino.app/meeting/abc"),
        LinkError::NotDeinoLink
    ));
    assert!(matches!(refused("not a url"), LinkError::NotDeinoLink));
    assert!(matches!(
        refused("deino://oauth/callback?code=x"),
        LinkError::UnknownRoute
    ));
    assert!(matches!(refused("deino://"), LinkError::UnknownRoute));
    let data = "a".repeat(MAX_LINK_CHARS);
    assert!(matches!(
        refused(&format!("deino://recipe?data={data}")),
        LinkError::TooLong
    ));
}

#[test]
fn routes_name_themselves_without_their_values() {
    assert_eq!(parse("deino://meeting/abc?t=1").route(), "meeting");
    assert_eq!(parse("deino://search?q=secret").route(), "search");
    assert_eq!(parse("deino://record/stop").route(), "record/stop");
}
