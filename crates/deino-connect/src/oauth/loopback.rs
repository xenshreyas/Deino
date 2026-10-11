//! The loopback redirect (RFC 8252 §7.3): a one-shot HTTP listener on this
//! Mac that receives the sign-in page's answer.
//!
//! It listens on 127.0.0.1 at a port the system picks, answers the
//! browser's request to `/callback` with a short "you can close this tab"
//! page, and hands back the code once the `state` matches. Anything else
//! that reaches the port (a favicon request) gets a 404 and is ignored.

use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use url::Url;

use super::OAuthError;

/// The path the sign-in page redirects to.
const CALLBACK_PATH: &str = "/callback";

/// The largest request head read from the browser.
const MAX_REQUEST_BYTES: usize = 16 * 1024;

/// What the browser shows after a successful sign-in.
const DONE_PAGE: &str = "<!doctype html><meta charset=utf-8><title>Deino</title>\
<body style=\"font-family:-apple-system,sans-serif;margin:4rem;color:#333\">\
<h1 style=\"font-size:1.25rem\">You are connected</h1><p>You can close this tab and return to Deino.</p>";

/// What the browser shows after a refused or broken sign-in.
const FAILED_PAGE: &str = "<!doctype html><meta charset=utf-8><title>Deino</title>\
<body style=\"font-family:-apple-system,sans-serif;margin:4rem;color:#333\">\
<h1 style=\"font-size:1.25rem\">Not connected</h1><p>Return to Deino to try again.</p>";

/// A listening loopback redirect.
#[derive(Debug)]
pub struct LoopbackRedirect {
    listener: TcpListener,
    redirect_uri: String,
}

impl LoopbackRedirect {
    /// Listens on 127.0.0.1 at a free port; `host` is how the redirect URI
    /// names it (`127.0.0.1` or `localhost`, as the provider registers it).
    pub async fn bind(host: &str) -> Result<Self, OAuthError> {
        let listener = TcpListener::bind(("127.0.0.1", 0))
            .await
            .map_err(OAuthError::Loopback)?;
        let port = listener.local_addr().map_err(OAuthError::Loopback)?.port();
        Ok(Self {
            listener,
            redirect_uri: format!("http://{host}:{port}{CALLBACK_PATH}"),
        })
    }

    /// The redirect URI to give the sign-in page.
    pub fn redirect_uri(&self) -> &str {
        &self.redirect_uri
    }

    /// Waits up to `timeout` for the redirect carrying `state`, and returns
    /// its authorization code.
    pub async fn receive(self, state: &str, timeout: Duration) -> Result<String, OAuthError> {
        tokio::time::timeout(timeout, self.accept_callback(state))
            .await
            .map_err(|_| OAuthError::TimedOut)?
    }

    async fn accept_callback(&self, state: &str) -> Result<String, OAuthError> {
        loop {
            let (mut stream, _) = self.listener.accept().await.map_err(OAuthError::Loopback)?;
            let Some(query) = read_callback(&mut stream).await? else {
                respond(&mut stream, "404 Not Found", "").await;
                continue;
            };
            let outcome = outcome_of(&query, state);
            let page = if outcome.is_ok() {
                DONE_PAGE
            } else {
                FAILED_PAGE
            };
            respond(&mut stream, "200 OK", page).await;
            return outcome;
        }
    }
}

/// The query of a `GET /callback` request; `None` for any other request.
async fn read_callback(stream: &mut TcpStream) -> Result<Option<Url>, OAuthError> {
    let mut head = Vec::new();
    let mut chunk = [0_u8; 1024];
    while !head.windows(4).any(|window| window == b"\r\n\r\n") && head.len() < MAX_REQUEST_BYTES {
        let read = stream
            .read(&mut chunk)
            .await
            .map_err(OAuthError::Loopback)?;
        if read == 0 {
            break;
        }
        head.extend_from_slice(chunk.get(..read).unwrap_or_default());
    }
    let head = String::from_utf8_lossy(&head);
    let target = head
        .lines()
        .next()
        .and_then(|line| line.strip_prefix("GET "))
        .and_then(|rest| rest.split(' ').next())
        .unwrap_or_default();
    let url = Url::parse(&format!("http://127.0.0.1{target}")).ok();
    Ok(url.filter(|url| url.path() == CALLBACK_PATH))
}

/// The code of a redirect, after checking its state; the provider's error
/// when it refused.
fn outcome_of(callback: &Url, state: &str) -> Result<String, OAuthError> {
    let value = |name: &str| {
        callback
            .query_pairs()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.into_owned())
    };
    if value("state").as_deref() != Some(state) {
        return Err(OAuthError::StateMismatch);
    }
    if let Some(reason) = value("error") {
        return Err(OAuthError::Denied {
            provider: "the provider",
            reason,
        });
    }
    value("code").ok_or(OAuthError::Denied {
        provider: "the provider",
        reason: "no code".to_owned(),
    })
}

/// Writes a small HTML answer and closes; the browser's view of it does not
/// change the outcome, so write errors are ignored.
async fn respond(stream: &mut TcpStream, status: &str, body: &str) {
    let response = format!(
        "HTTP/1.1 {status}\r\ncontent-type: text/html; charset=utf-8\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(response.as_bytes()).await.ok();
    stream.shutdown().await.ok();
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Sends one GET to the redirect and returns the HTTP status line.
    async fn get(uri: &str, path_and_query: &str) -> String {
        let authority = uri.trim_start_matches("http://").split('/').next().unwrap();
        let port = authority.rsplit(':').next().unwrap();
        let mut stream = TcpStream::connect(("127.0.0.1", port.parse::<u16>().unwrap()))
            .await
            .unwrap();
        let request = format!("GET {path_and_query} HTTP/1.1\r\nhost: {authority}\r\n\r\n");
        stream.write_all(request.as_bytes()).await.unwrap();
        let mut answer = String::new();
        stream.read_to_string(&mut answer).await.unwrap();
        answer.lines().next().unwrap().to_owned()
    }

    #[tokio::test]
    async fn the_code_arrives_after_a_stray_request_and_the_browser_is_answered() {
        let redirect = LoopbackRedirect::bind("127.0.0.1").await.unwrap();
        let uri = redirect.redirect_uri().to_owned();
        assert!(uri.starts_with("http://127.0.0.1:") && uri.ends_with("/callback"));

        let browser = async {
            let favicon = get(&uri, "/favicon.ico").await;
            let callback = get(&uri, "/callback?code=abc%2F1&state=s1").await;
            (favicon, callback)
        };
        let (code, (favicon, callback)) =
            tokio::join!(redirect.receive("s1", Duration::from_secs(5)), browser);

        assert_eq!(code.unwrap(), "abc/1");
        assert_eq!(favicon, "HTTP/1.1 404 Not Found");
        assert_eq!(callback, "HTTP/1.1 200 OK");
    }

    #[tokio::test]
    async fn a_foreign_state_or_a_refusal_is_not_a_code() {
        let forged = Url::parse("http://127.0.0.1/callback?code=x&state=other").unwrap();
        let refused = Url::parse("http://127.0.0.1/callback?error=access_denied&state=s1").unwrap();

        assert!(matches!(
            outcome_of(&forged, "s1"),
            Err(OAuthError::StateMismatch)
        ));
        assert!(matches!(
            outcome_of(&refused, "s1"),
            Err(OAuthError::Denied { reason, .. }) if reason == "access_denied"
        ));
    }

    #[tokio::test(start_paused = true)]
    async fn nobody_signing_in_times_out() {
        let redirect = LoopbackRedirect::bind("localhost").await.unwrap();

        let outcome = redirect.receive("s1", Duration::from_mins(5)).await;

        assert!(matches!(outcome, Err(OAuthError::TimedOut)));
    }
}
