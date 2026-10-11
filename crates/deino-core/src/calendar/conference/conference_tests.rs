//! The conference parser against invitation formats seen in the wild: each
//! case is the field an invitation carries its link in, and the link and
//! service Deino must find.

use super::*;

/// Which field of the event a case fills.
#[derive(Clone, Copy, Debug)]
enum Field {
    Conference,
    Url,
    Location,
    Notes,
}

/// One invitation: `text` in `field`, and the expected join link.
struct Case {
    name: &'static str,
    field: Field,
    text: &'static str,
    expected: Option<(&'static str, MeetingService)>,
}

const fn case(
    name: &'static str,
    field: Field,
    text: &'static str,
    expected: Option<(&'static str, MeetingService)>,
) -> Case {
    Case {
        name,
        field,
        text,
        expected,
    }
}

use Field::{Conference as Data, Location, Notes, Url};
use MeetingService as S;

const CASES: &[Case] = &[
    case(
        "zoom invite body with password and dial-in",
        Notes,
        "Ada Lovelace is inviting you to a scheduled Zoom meeting.\n\nJoin Zoom Meeting\nhttps://us02web.zoom.us/j/88123456789?pwd=QWxhZGRpbjpvcGVuIHNlc2FtZQ.1\n\nMeeting ID: 881 2345 6789\nPasscode: 123456\nOne tap mobile\n+13017158592,,88123456789#",
        Some((
            "https://us02web.zoom.us/j/88123456789?pwd=QWxhZGRpbjpvcGVuIHNlc2FtZQ.1",
            S::Zoom,
        )),
    ),
    case(
        "zoom link in the location field",
        Location,
        "https://acme.zoom.us/j/95551234567",
        Some(("https://acme.zoom.us/j/95551234567", S::Zoom)),
    ),
    case(
        "zoom personal meeting room",
        Url,
        "https://zoom.us/my/ada.lovelace",
        Some(("https://zoom.us/my/ada.lovelace", S::Zoom)),
    ),
    case(
        "zoomgov webinar",
        Notes,
        "Please join: https://acme.zoomgov.com/w/1601234567?tk=abc",
        Some(("https://acme.zoomgov.com/w/1601234567?tk=abc", S::Zoom)),
    ),
    case(
        "zoom native client link",
        Location,
        "zoommtg://zoom.us/join?action=join&confno=88123456789",
        Some((
            "zoommtg://zoom.us/join?action=join&confno=88123456789",
            S::Zoom,
        )),
    ),
    case(
        "bare zoom link without a scheme",
        Location,
        "Conf room B / zoom.us/j/5551234",
        Some(("https://zoom.us/j/5551234", S::Zoom)),
    ),
    case(
        "google meet conference data",
        Data,
        "https://meet.google.com/abc-defg-hij",
        Some(("https://meet.google.com/abc-defg-hij", S::GoogleMeet)),
    ),
    case(
        "google meet in a google calendar description",
        Notes,
        "Join with Google Meet: https://meet.google.com/xyz-abcd-efg?hs=122&authuser=0\nOr dial: (US) +1 470-273-8742 PIN: 123456789#",
        Some((
            "https://meet.google.com/xyz-abcd-efg?hs=122&authuser=0",
            S::GoogleMeet,
        )),
    ),
    case(
        "google meet nickname link",
        Location,
        "https://g.co/meet/team-standup",
        Some(("https://g.co/meet/team-standup", S::GoogleMeet)),
    ),
    case(
        "classic teams meetup-join link in outlook html",
        Notes,
        r#"<div><a href="https://teams.microsoft.com/l/meetup-join/19%3ameeting_NjA1YTdmZjMtNDk2%40thread.v2/0?context=%7b%22Tid%22%3a%2272f988bf%22%7d" target="_blank">Click here to join the meeting</a></div>"#,
        Some((
            "https://teams.microsoft.com/l/meetup-join/19%3ameeting_NjA1YTdmZjMtNDk2%40thread.v2/0?context=%7b%22Tid%22%3a%2272f988bf%22%7d",
            S::Teams,
        )),
    ),
    case(
        "new short teams link with passcode",
        Notes,
        "Microsoft Teams Need help?\nJoin the meeting now <https://teams.microsoft.com/meet/2468013579?p=AbCdEfGh123>\nMeeting ID: 246 801 357 9",
        Some((
            "https://teams.microsoft.com/meet/2468013579?p=AbCdEfGh123",
            S::Teams,
        )),
    ),
    case(
        "teams for personal accounts",
        Url,
        "https://teams.live.com/meet/9876543210123?p=xyz",
        Some(("https://teams.live.com/meet/9876543210123?p=xyz", S::Teams)),
    ),
    case(
        "teams government cloud",
        Notes,
        "Join: https://gov.teams.microsoft.us/l/meetup-join/19%3ameeting_abc%40thread.v2/0",
        Some((
            "https://gov.teams.microsoft.us/l/meetup-join/19%3ameeting_abc%40thread.v2/0",
            S::Teams,
        )),
    ),
    case(
        "teams link behind outlook safe links",
        Notes,
        "Join: https://nam12.safelinks.protection.outlook.com/?url=https%3A%2F%2Fteams.microsoft.com%2Fl%2Fmeetup-join%2F19%253ameeting_abc%2540thread.v2%2F0&data=05%7C01&sdata=x&reserved=0",
        Some((
            "https://teams.microsoft.com/l/meetup-join/19%3ameeting_abc%40thread.v2/0",
            S::Teams,
        )),
    ),
    case(
        "teams location text has no link, the notes do",
        Location,
        "Microsoft Teams Meeting",
        None,
    ),
    case(
        "webex personal room",
        Notes,
        "Join from the meeting link\nhttps://acme.webex.com/meet/ada.lovelace\n\nJoin by meeting number",
        Some(("https://acme.webex.com/meet/ada.lovelace", S::Webex)),
    ),
    case(
        "webex scheduled meeting",
        Location,
        "https://acme.webex.com/acme/j.php?MTID=m1234567890abcdef",
        Some((
            "https://acme.webex.com/acme/j.php?MTID=m1234567890abcdef",
            S::Webex,
        )),
    ),
    case(
        "slack huddle link",
        Url,
        "https://app.slack.com/huddle/T01ABCDEF/C02GHIJKL",
        Some((
            "https://app.slack.com/huddle/T01ABCDEF/C02GHIJKL",
            S::SlackHuddle,
        )),
    ),
    case(
        "goto meeting",
        Notes,
        "Please join my meeting from your computer, tablet or smartphone.\nhttps://meet.goto.com/123456789\n\nYou can also dial in using your phone.",
        Some(("https://meet.goto.com/123456789", S::GoTo)),
    ),
    case(
        "legacy gotomeeting join link",
        Location,
        "https://global.gotomeeting.com/join/987654321",
        Some(("https://global.gotomeeting.com/join/987654321", S::GoTo)),
    ),
    case(
        "jitsi room",
        Location,
        "https://meet.jit.si/DeinoWeeklySync",
        Some(("https://meet.jit.si/DeinoWeeklySync", S::Jitsi)),
    ),
    case(
        "whereby room",
        Notes,
        "Room: https://whereby.com/deino-design.",
        Some(("https://whereby.com/deino-design", S::Whereby)),
    ),
    case(
        "amazon chime meeting",
        Notes,
        "Click to join the meeting: https://chime.aws/1234567890\nMeeting ID: 1234 56 7890",
        Some(("https://chime.aws/1234567890", S::Chime)),
    ),
    case(
        "around room",
        Url,
        "https://meet.around.co/r/deino-sync",
        Some(("https://meet.around.co/r/deino-sync", S::Around)),
    ),
    case(
        "facetime link",
        Notes,
        "FaceTime: https://facetime.apple.com/join#v=1&p=AbCdEf&k=XyZ",
        Some((
            "https://facetime.apple.com/join#v=1&p=AbCdEf&k=XyZ",
            S::FaceTime,
        )),
    ),
    case(
        "google redirect around a zoom link",
        Notes,
        "https://www.google.com/url?q=https://zoom.us/j/111222333&sa=D&source=calendar",
        Some(("https://zoom.us/j/111222333", S::Zoom)),
    ),
    case(
        "proofpoint-wrapped zoom link",
        Notes,
        "https://urldefense.com/v3/__https://zoom.us/j/444555666__;!!Abc123$",
        Some(("https://zoom.us/j/444555666", S::Zoom)),
    ),
    case(
        "a document link before the meeting link is skipped",
        Notes,
        "Agenda: https://docs.google.com/document/d/1Hq3/edit\nJoin: https://meet.google.com/qrs-tuvw-xyz",
        Some(("https://meet.google.com/qrs-tuvw-xyz", S::GoogleMeet)),
    ),
    case(
        "zoom help pages are not meetings",
        Notes,
        "Trouble joining? https://support.zoom.us/hc/en-us and https://zoom.us/download",
        None,
    ),
    case(
        "the google meet home page is not a meeting",
        Url,
        "https://meet.google.com/",
        None,
    ),
    case(
        "an in-person event has no link",
        Location,
        "Blue room, Deino office",
        None,
    ),
    case(
        "an email address on a meeting host is not a link",
        Notes,
        "Questions? Write to events@zoom.us",
        None,
    ),
];

fn fields(case: &Case) -> ConferenceFields<'static> {
    let text = Some(case.text);
    match case.field {
        Data => ConferenceFields {
            conference: text,
            ..ConferenceFields::default()
        },
        Url => ConferenceFields {
            url: text,
            ..ConferenceFields::default()
        },
        Location => ConferenceFields {
            location: text,
            ..ConferenceFields::default()
        },
        Notes => ConferenceFields {
            notes: text,
            ..ConferenceFields::default()
        },
    }
}

#[test]
fn every_invitation_format_yields_its_join_link() {
    for case in CASES {
        let found = find_conference(fields(case));
        let found = found
            .as_ref()
            .map(|conference| (conference.url.as_str(), conference.service));
        assert_eq!(found, case.expected, "{} ({:?})", case.name, case.field);
    }
}

#[test]
fn structured_conference_data_beats_a_link_in_the_notes() {
    let found = find_conference(ConferenceFields {
        conference: Some("https://meet.google.com/abc-defg-hij"),
        notes: Some("Backup: https://zoom.us/j/1"),
        ..ConferenceFields::default()
    });

    assert_eq!(found.unwrap().service, MeetingService::GoogleMeet);
}

#[test]
fn a_known_service_later_in_the_list_beats_an_unknown_link_earlier() {
    let found = find_conference(ConferenceFields {
        url: Some("https://acme.com/meetings/weekly"),
        notes: Some("https://zoom.us/j/1"),
        ..ConferenceFields::default()
    });

    assert_eq!(found.unwrap().url, "https://zoom.us/j/1");
}

#[test]
fn only_the_big_five_have_their_own_platform_value() {
    assert_eq!(
        MeetingService::GoogleMeet.platform(),
        ConferencePlatform::GoogleMeet
    );
    assert_eq!(
        MeetingService::SlackHuddle.platform(),
        ConferencePlatform::Slack
    );
    assert_eq!(MeetingService::GoTo.platform(), ConferencePlatform::Other);
    assert_eq!(MeetingService::Jitsi.platform(), ConferencePlatform::Other);
}
