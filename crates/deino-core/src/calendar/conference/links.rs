//! Finding links in free text: event notes (plain text or HTML), locations
//! and URL fields, as calendar apps and mail gateways write them.

use url::Url;

/// Schemes a join link can start with; matched case-insensitively.
const SCHEMES: &[&str] = &["https://", "http://", "zoommtg://", "zoomus://", "msteams:"];

/// How many redirect wrappers (a mail gateway inside a mail gateway) are
/// peeled off one link at most.
const MAX_UNWRAPS: usize = 3;

/// Characters that end a link in prose, HTML attributes and Markdown.
fn ends_link(c: char) -> bool {
    c.is_whitespace()
        || matches!(
            c,
            '"' | '\'' | '<' | '>' | '`' | '|' | '\\' | '{' | '}' | '[' | ']'
        )
}

/// Every link in `text`, in order: explicit links first, then bare
/// `host/path` tokens (`zoom.us/j/123`) when the text has no explicit link.
pub(super) fn links_in(text: &str) -> Vec<Url> {
    let explicit: Vec<Url> = scheme_links(text).filter_map(|raw| parse(&raw)).collect();
    if !explicit.is_empty() {
        return explicit;
    }
    text.split(ends_link)
        .filter(|token| looks_like_bare_link(token))
        .filter_map(|token| parse(&format!("https://{}", clean(token))))
        .collect()
}

/// The raw text of every link that starts with a known scheme.
fn scheme_links(text: &str) -> impl Iterator<Item = String> + '_ {
    // ASCII lower-casing keeps byte offsets, so indices found in `lower`
    // are valid in `text`.
    let lower = text.to_ascii_lowercase();
    let mut starts: Vec<usize> = SCHEMES
        .iter()
        .flat_map(|scheme| lower.match_indices(scheme).map(|(index, _)| index))
        .collect();
    starts.sort_unstable();
    starts.dedup();
    starts.into_iter().filter_map(move |start| {
        let rest = text.get(start..)?;
        let end = rest.find(ends_link).unwrap_or(rest.len());
        Some(clean(rest.get(..end)?))
    })
}

/// True for a token such as `us02web.zoom.us/j/123`: a dotted host, then a path.
fn looks_like_bare_link(token: &str) -> bool {
    let Some((host, path)) = token.split_once('/') else {
        return false;
    };
    !path.is_empty()
        && host.contains('.')
        && !host.contains('@')
        && host
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
}

/// Strips what prose wraps around a link: trailing punctuation, an
/// unbalanced closing parenthesis, and HTML-escaped ampersands.
fn clean(raw: &str) -> String {
    let mut link = raw.replace("&amp;", "&");
    while let Some(last) = link.chars().last() {
        let unbalanced = last == ')' && link.matches('(').count() < link.matches(')').count();
        if !matches!(last, '.' | ',' | ';' | ':' | '!' | '?' | '*') && !unbalanced {
            break;
        }
        link.pop();
    }
    link
}

/// Parses `raw`, then peels off redirect wrappers.
fn parse(raw: &str) -> Option<Url> {
    let mut url = Url::parse(raw).ok()?;
    for _ in 0..MAX_UNWRAPS {
        match unwrap_redirect(&url) {
            Some(inner) => url = inner,
            None => break,
        }
    }
    Some(url)
}

/// The link a mail-security or search redirect points at: Outlook Safe
/// Links (`?url=`), Google's `/url?q=` and Proofpoint's `v3/__<link>__`.
fn unwrap_redirect(url: &Url) -> Option<Url> {
    let host = url.host_str()?.to_ascii_lowercase();
    if host.ends_with("safelinks.protection.outlook.com") {
        return query_link(url, &["url"]);
    }
    if (host == "www.google.com" || host == "google.com") && url.path() == "/url" {
        return query_link(url, &["q", "url"]);
    }
    if host == "urldefense.com" {
        let inner = url.as_str().split_once("/v3/__")?.1.split_once("__")?.0;
        return Url::parse(inner).ok();
    }
    None
}

/// The first of `names` among `url`'s query parameters, parsed as a link.
fn query_link(url: &Url, names: &[&str]) -> Option<Url> {
    url.query_pairs()
        .find(|(name, _)| names.contains(&name.as_ref()))
        .and_then(|(_, value)| Url::parse(&value).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn found(text: &str) -> Vec<String> {
        links_in(text).iter().map(ToString::to_string).collect()
    }

    #[test]
    fn links_end_at_whitespace_quotes_and_angle_brackets() {
        assert_eq!(
            found(
                r#"Join: <https://zoom.us/j/1> or <a href="https://meet.google.com/abc-defg-hij">"#
            ),
            [
                "https://zoom.us/j/1",
                "https://meet.google.com/abc-defg-hij"
            ]
        );
    }

    #[test]
    fn trailing_punctuation_and_unbalanced_parentheses_are_dropped() {
        assert_eq!(
            found("(see https://zoom.us/j/123?pwd=x.)"),
            ["https://zoom.us/j/123?pwd=x"]
        );
        assert_eq!(
            found("https://en.wikipedia.org/wiki/Foo_(bar)"),
            ["https://en.wikipedia.org/wiki/Foo_(bar)"]
        );
    }

    #[test]
    fn bare_links_count_only_when_there_is_no_explicit_one() {
        assert_eq!(found("Room 4 / zoom.us/j/55"), ["https://zoom.us/j/55"]);
        assert_eq!(found("ada@zoom.us"), Vec::<String>::new());
    }

    #[test]
    fn html_escaped_ampersands_are_restored() {
        assert_eq!(
            found("https://zoom.us/j/1?pwd=a&amp;uname=b"),
            ["https://zoom.us/j/1?pwd=a&uname=b"]
        );
    }
}
