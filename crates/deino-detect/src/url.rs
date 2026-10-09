//! Meeting URLs: which platform a browser tab's address belongs to, and a
//! stable identity for its room, so one call is one detection.
//!
//! Ported from Muesli's `MeetingURLNormalizer` (MIT): Google Meet room codes
//! (`abc-defg-hij`), Zoom `/j/<id>` and web-client links, Teams, Webex,
//! FaceTime links and Slack huddles. Query strings and fragments never take
//! part, so a refreshed tab stays the same meeting. URLs are never logged.

use crate::model::Platform;

/// A browser tab that is a meeting room.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MeetingUrl {
    /// The service.
    pub platform: Platform,
    /// The room's identity, host and path without query or fragment, for
    /// example `meet.google.com/abc-defg-hij`.
    pub room: String,
}

/// The meeting `raw` points at, or `None` for any other page.
pub fn normalize(raw: &str) -> Option<MeetingUrl> {
    let (host, segments) = split(raw)?;
    let platform = platform_of(&host, &segments)?;
    let room = match platform {
        Platform::GoogleMeet => format!("{host}/{}", segments.first()?.to_lowercase()),
        Platform::Zoom => zoom_room(&host, &segments),
        _ => compact(&host, &segments),
    };
    Some(MeetingUrl { platform, room })
}

/// The platform of a page, or `None` when it is not a meeting room.
fn platform_of(host: &str, segments: &[&str]) -> Option<Platform> {
    if host == "meet.google.com" {
        return segments
            .first()
            .is_some_and(|code| is_meet_code(code))
            .then_some(Platform::GoogleMeet);
    }
    if host_is(host, "zoom.us") {
        return Some(Platform::Zoom);
    }
    if host_is(host, "teams.microsoft.com")
        || host == "teams.live.com"
        || host == "teams.cloud.microsoft"
    {
        return Some(Platform::Teams);
    }
    if host_is(host, "webex.com") {
        return Some(Platform::Webex);
    }
    if host == "facetime.apple.com" {
        return Some(Platform::FaceTime);
    }
    (host == "app.slack.com" && segments.first() == Some(&"huddle")).then_some(Platform::Slack)
}

/// The lower-case host and the non-empty path segments of `raw`.
fn split(raw: &str) -> Option<(String, Vec<&str>)> {
    let trimmed = raw.trim();
    let without_scheme = trimmed.split_once("://").map_or(trimmed, |(_, rest)| rest);
    let address = without_scheme.split(['?', '#']).next()?;
    let (authority, path) = address.split_once('/').unwrap_or((address, ""));
    let host = authority
        .rsplit('@')
        .next()?
        .split(':')
        .next()?
        .to_ascii_lowercase();
    if host.is_empty() {
        return None;
    }
    Some((
        host,
        path.split('/')
            .filter(|segment| !segment.is_empty())
            .collect(),
    ))
}

/// `xxx-xxxx-xxx`, lower-case letters only.
fn is_meet_code(code: &str) -> bool {
    let parts: Vec<&str> = code.split('-').collect();
    let shape = parts.iter().map(|part| part.len()).collect::<Vec<_>>() == [3, 4, 3];
    shape
        && parts
            .iter()
            .all(|part| part.chars().all(|c| c.is_ascii_alphabetic()))
}

/// `zoom.us/j/<id>` for join links and web-client links alike, so the app's
/// and the browser's view of one meeting agree.
fn zoom_room(host: &str, segments: &[&str]) -> String {
    zoom_meeting_id(segments).map_or_else(|| compact(host, segments), |id| format!("{host}/j/{id}"))
}

/// The meeting number after `/j/`, `/s/`, `/wc/` or `/wc/join/`.
fn zoom_meeting_id<'a>(segments: &[&'a str]) -> Option<&'a str> {
    let (prefix, rest) = segments.split_first()?;
    if !["j", "s", "wc"].contains(prefix) {
        return None;
    }
    let rest = rest.strip_prefix(&["join"]).unwrap_or(rest);
    rest.first()
        .copied()
        .filter(|id| !id.is_empty() && id.chars().all(|c| c.is_ascii_digit()))
}

fn compact(host: &str, segments: &[&str]) -> String {
    if segments.is_empty() {
        host.to_owned()
    } else {
        format!("{host}/{}", segments.join("/"))
    }
}

/// `host` is `domain` or one of its subdomains.
fn host_is(host: &str, domain: &str) -> bool {
    host == domain
        || host
            .strip_suffix(domain)
            .is_some_and(|sub| sub.ends_with('.'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[track_caller]
    fn assert_room(raw: &str, platform: Platform, room: &str) {
        let expected = MeetingUrl {
            platform,
            room: room.to_owned(),
        };
        assert_eq!(normalize(raw), Some(expected), "{raw}");
    }

    #[test]
    fn meet_rooms_need_a_room_code_and_ignore_query_and_case() {
        assert_room(
            "https://meet.google.com/ABC-defg-hij?authuser=1#x",
            Platform::GoogleMeet,
            "meet.google.com/abc-defg-hij",
        );
        for not_a_room in [
            "https://meet.google.com/landing",
            "https://meet.google.com/",
            "meet.google.com/abc-defg-hi1",
        ] {
            assert_eq!(normalize(not_a_room), None, "{not_a_room}");
        }
    }

    #[test]
    fn zoom_join_and_web_client_links_are_the_same_room() {
        for link in [
            "https://us02web.zoom.us/j/81234567890?pwd=secret",
            "https://us02web.zoom.us/wc/81234567890/join",
            "https://us02web.zoom.us/wc/join/81234567890",
        ] {
            assert_room(link, Platform::Zoom, "us02web.zoom.us/j/81234567890");
        }
        assert_room("https://zoom.us/my/ada", Platform::Zoom, "zoom.us/my/ada");
        assert_eq!(normalize("https://notzoom.us/j/1"), None);
    }

    #[test]
    fn teams_webex_facetime_and_slack_huddles_are_recognised() {
        assert_room(
            "https://teams.microsoft.com/l/meetup-join/19%3ameeting/0?context=x",
            Platform::Teams,
            "teams.microsoft.com/l/meetup-join/19%3ameeting/0",
        );
        assert_room(
            "https://teams.live.com/meet/9876",
            Platform::Teams,
            "teams.live.com/meet/9876",
        );
        assert_room(
            "https://acme.webex.com/meet/ada",
            Platform::Webex,
            "acme.webex.com/meet/ada",
        );
        assert_room(
            "https://facetime.apple.com/join#v=1&p=abc",
            Platform::FaceTime,
            "facetime.apple.com/join",
        );
        assert_room(
            "https://app.slack.com/huddle/T0123/C0456",
            Platform::Slack,
            "app.slack.com/huddle/T0123/C0456",
        );
    }

    #[test]
    fn ordinary_pages_are_not_meetings() {
        for page in [
            "https://app.slack.com/client/T0123/C0456",
            "https://www.google.com/search?q=meet.google.com",
            "https://example.com/j/123",
            "",
            "https://",
        ] {
            assert_eq!(normalize(page), None, "{page}");
        }
    }
}
