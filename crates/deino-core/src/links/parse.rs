//! `DeepLink::parse`: a `deino://` URL to a validated [`DeepLink`].
//!
//! The route is the URL's host and path together, so `deino://meeting/<id>`
//! and `deino:///meeting/<id>` read the same. Route names ignore case;
//! unknown query parameters are ignored; a known one that is malformed
//! refuses the whole link.

use std::str::FromStr;

use deino_domain::{CalendarEventId, MeetingId, Millis};
use url::Url;

use super::error::LinkError;
use super::model::{DeepLink, MAX_LINK_CHARS, SCHEME};

/// The longest meeting ID a link may carry.
const MAX_MEETING_ID_CHARS: usize = 128;
/// The longest search query a link may carry.
const MAX_QUERY_CHARS: usize = 1_000;

impl DeepLink {
    /// Parses and validates `text`, a whole `deino://` URL.
    pub fn parse(text: &str) -> Result<Self, LinkError> {
        if text.len() > MAX_LINK_CHARS {
            return Err(LinkError::TooLong);
        }
        let url = Url::parse(text.trim()).map_err(|_| LinkError::NotDeinoLink)?;
        if url.scheme() != SCHEME {
            return Err(LinkError::NotDeinoLink);
        }
        let route = route(&url);
        let route: Vec<&str> = route.iter().map(String::as_str).collect();
        match route.as_slice() {
            ["meeting" | "meetings", id] => meeting(id, &url),
            ["new-note"] => new_note(&url),
            ["record", "start"] => Ok(Self::RecordStart),
            ["record", "stop"] => Ok(Self::RecordStop),
            ["search"] => search(&url),
            ["recipe"] => recipe(&url),
            _ => Err(LinkError::UnknownRoute),
        }
    }
}

/// The host and the path's non-empty segments. Route names are lowercased;
/// the last segment of a meeting route keeps its case (it is the ID).
fn route(url: &Url) -> Vec<String> {
    let host = url.host_str().into_iter();
    let path = url.path_segments().into_iter().flatten();
    let mut parts: Vec<String> = host
        .chain(path)
        .filter(|part| !part.is_empty())
        .map(str::to_owned)
        .collect();
    let meeting_route = parts.len() == 2 && parts.first().is_some_and(|name| is_meeting(name));
    let names = if meeting_route { 1 } else { parts.len() };
    for part in parts.iter_mut().take(names) {
        part.make_ascii_lowercase();
    }
    parts
}

fn is_meeting(name: &str) -> bool {
    name.eq_ignore_ascii_case("meeting") || name.eq_ignore_ascii_case("meetings")
}

/// The first value of query parameter `key`, decoded.
fn param(url: &Url, key: &str) -> Option<String> {
    url.query_pairs()
        .find(|(name, _)| name == key)
        .map(|(_, value)| value.into_owned())
}

fn meeting(id: &str, url: &Url) -> Result<DeepLink, LinkError> {
    let valid = !id.is_empty()
        && id.len() <= MAX_MEETING_ID_CHARS
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if !valid {
        return Err(LinkError::Invalid("meeting ID"));
    }
    let at = param(url, "t")
        .map(|t| t.parse::<u32>().map(Millis))
        .transpose()
        .map_err(|_| LinkError::Invalid("moment (t)"))?;
    Ok(DeepLink::Meeting {
        id: MeetingId::new(id),
        at,
    })
}

fn new_note(url: &Url) -> Result<DeepLink, LinkError> {
    let event = param(url, "event")
        .map(|event| CalendarEventId::from_str(&event))
        .transpose()
        .map_err(|_| LinkError::Invalid("calendar event"))?;
    Ok(DeepLink::NewNote { event })
}

fn search(url: &Url) -> Result<DeepLink, LinkError> {
    let query = param(url, "q").unwrap_or_default();
    let query = query.trim();
    if query.is_empty() {
        return Err(LinkError::Missing("search query (q)"));
    }
    if query.chars().count() > MAX_QUERY_CHARS {
        return Err(LinkError::Invalid("search query (q)"));
    }
    Ok(DeepLink::Search {
        query: query.to_owned(),
    })
}

fn recipe(url: &Url) -> Result<DeepLink, LinkError> {
    let data = param(url, "data").unwrap_or_default();
    if data.trim().is_empty() {
        return Err(LinkError::Missing("recipe (data)"));
    }
    if data.chars().any(char::is_control) {
        return Err(LinkError::Invalid("recipe (data)"));
    }
    Ok(DeepLink::Recipe { data })
}
