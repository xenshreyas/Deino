//! Reading the browser's redirect and answering it.
//!
//! The listener speaks just enough HTTP/1.1 for one `GET`: the request line
//! of a head of at most [`MAX_HEAD_BYTES`], then a complete response with
//! `Connection: close`. No other HTTP is served.

use url::Url;

/// The largest request head the listener reads: a redirect URL with a long
/// code and state, plus a browser's headers.
pub(super) const MAX_HEAD_BYTES: usize = 16 * 1024;

/// What the redirect carried.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Callback {
    /// The user approved: the code, for the state it was sent with.
    Code {
        /// The authorization code.
        code: String,
        /// The `state` echoed back.
        state: String,
    },
    /// The provider sent an error (`access_denied`, ...), for this state.
    Error {
        /// The OAuth error code.
        error: String,
        /// The `state` echoed back, if any.
        state: Option<String>,
    },
    /// A request for something else (`/favicon.ico`, a stray probe).
    Other,
}

/// Parses the request head `head` as a redirect to `path`.
pub(super) fn parse(head: &[u8], path: &str) -> Callback {
    let Some(target) = request_target(head) else {
        return Callback::Other;
    };
    let Ok(url) = Url::parse("http://127.0.0.1").and_then(|base| base.join(target)) else {
        return Callback::Other;
    };
    if url.path() != path {
        return Callback::Other;
    }
    let value = |name: &str| {
        url.query_pairs()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.into_owned())
    };
    match (value("code"), value("state"), value("error")) {
        (_, state, Some(error)) => Callback::Error { error, state },
        (Some(code), Some(state), None) => Callback::Code { code, state },
        _ => Callback::Other,
    }
}

/// The target of a `GET` request line: `GET /callback?code=… HTTP/1.1`.
fn request_target(head: &[u8]) -> Option<&str> {
    let line_end = head.windows(2).position(|pair| pair == b"\r\n")?;
    let line = std::str::from_utf8(head.get(..line_end)?).ok()?;
    let mut parts = line.split(' ');
    let (method, target, version) = (parts.next()?, parts.next()?, parts.next()?);
    let valid = method == "GET" && target.starts_with('/') && version.starts_with("HTTP/1.");
    valid.then_some(target)
}

/// True once `head` holds a whole request head.
pub(super) fn is_complete(head: &[u8]) -> bool {
    head.windows(4).any(|window| window == b"\r\n\r\n")
}

/// What the browser shows after the redirect.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Page {
    /// The sign-in reached Deino.
    Done,
    /// The provider reported an error, or the request was not ours.
    Failed,
    /// Not the redirect path.
    NotFound,
}

/// A complete HTTP response for `page`.
pub(super) fn response(page: Page) -> String {
    let (status, title, body) = match page {
        Page::Done => (
            "200 OK",
            "Back to Deino",
            "Deino received the sign-in. You can close this tab.",
        ),
        Page::Failed => (
            "400 Bad Request",
            "Not connected",
            "The sign-in did not complete. Return to Deino to try again.",
        ),
        Page::NotFound => ("404 Not Found", "Not found", "Nothing here."),
    };
    let html = format!(
        "<!doctype html><meta charset=utf-8><title>{title}</title>\
         <body style=\"font:16px -apple-system,system-ui,sans-serif;margin:20vh auto;\
         max-width:28rem;padding:0 1.5rem;color:#2b2a27;background:#faf9f7\">\
         <h1 style=\"font-size:24px;font-weight:600\">{title}</h1><p>{body}</p>"
    );
    format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\n\
         Content-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n{html}",
        html.len()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn head(target: &str) -> Vec<u8> {
        format!("GET {target} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n").into_bytes()
    }

    #[test]
    fn a_redirect_with_a_code_and_state_is_read() {
        let callback = parse(&head("/callback?code=abc%2F1&state=s1"), "/callback");
        assert_eq!(
            callback,
            Callback::Code {
                code: "abc/1".into(),
                state: "s1".into()
            }
        );
    }

    #[test]
    fn a_provider_error_wins_over_a_code() {
        let callback = parse(&head("/callback?error=access_denied&state=s1"), "/callback");
        assert_eq!(
            callback,
            Callback::Error {
                error: "access_denied".into(),
                state: Some("s1".into())
            }
        );
    }

    #[test]
    fn anything_else_is_not_a_callback() {
        for request in [
            head("/favicon.ico"),
            head("/callback"),
            head("/callback?code=only"),
            b"POST /callback?code=a&state=b HTTP/1.1\r\n\r\n".to_vec(),
            b"garbage".to_vec(),
        ] {
            assert_eq!(parse(&request, "/callback"), Callback::Other);
        }
    }

    #[test]
    fn responses_declare_their_length_and_close() {
        let done = response(Page::Done);
        let (head, body) = done.split_once("\r\n\r\n").unwrap();

        assert!(head.starts_with("HTTP/1.1 200 OK"));
        assert!(head.contains(&format!("Content-Length: {}", body.len())));
        assert!(head.contains("Connection: close"));
        assert!(is_complete(done.as_bytes()));
    }
}
