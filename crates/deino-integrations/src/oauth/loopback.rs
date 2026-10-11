//! [`LoopbackListener`]: where the browser comes back to after a sign-in
//! (RFC 8252 §7.3).
//!
//! It listens on `127.0.0.1` only (and `[::1]` for `localhost` redirects),
//! so nothing off this Mac can reach it, serves one redirect, and is closed
//! when dropped. A request whose `state` does not match is answered and
//! ignored, so a stray page cannot end (or hijack) the sign-in.

mod callback;

use std::io;
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio_util::sync::CancellationToken;
use url::Url;

use self::callback::{Callback, MAX_HEAD_BYTES, Page};
use super::{OAuthError, error_code};
use crate::{LoopbackHost, PortRule, RedirectSpec};

/// How long one browser connection may take to send its request.
const READ_TIMEOUT: Duration = Duration::from_secs(10);

/// A bound loopback listener and the redirect URI that reaches it.
#[derive(Debug)]
pub struct LoopbackListener {
    ipv4: TcpListener,
    ipv6: Option<TcpListener>,
    redirect_uri: Url,
    path: &'static str,
}

impl LoopbackListener {
    /// Binds the listener `spec` describes; `port` replaces a fixed port
    /// (the user registered another one).
    pub async fn bind(spec: RedirectSpec, port: Option<u16>) -> Result<Self, OAuthError> {
        let wanted = match spec.port {
            PortRule::Any => 0,
            PortRule::Fixed(default) => port.unwrap_or(default),
        };
        let ipv4 = TcpListener::bind((Ipv4Addr::LOCALHOST, wanted))
            .await
            .map_err(|error| bind_error(wanted, error))?;
        let bound = ipv4.local_addr().map_err(OAuthError::Listen)?.port();
        // Browsers may resolve `localhost` to `::1`; best effort, since a Mac
        // without IPv6 still reaches 127.0.0.1.
        let ipv6 = match spec.host {
            LoopbackHost::Localhost => TcpListener::bind((Ipv6Addr::LOCALHOST, bound)).await.ok(),
            LoopbackHost::Ipv4 => None,
        };
        let address = format!("http://{}:{bound}{}", spec.host.as_str(), spec.path);
        let redirect_uri = Url::parse(&address).map_err(|_| OAuthError::BadEndpoint)?;
        Ok(Self {
            ipv4,
            ipv6,
            redirect_uri,
            path: spec.path,
        })
    }

    /// The redirect URI to send in the authorization request.
    pub fn redirect_uri(&self) -> &Url {
        &self.redirect_uri
    }

    /// Waits for the redirect carrying `state` and returns its code. Stops
    /// with [`OAuthError::TimedOut`] after `timeout`, and with
    /// [`OAuthError::AccessDenied`] or [`OAuthError::Provider`] when the
    /// provider reports an error for this sign-in.
    pub async fn wait(
        self,
        state: &str,
        cancel: &CancellationToken,
        timeout: Duration,
    ) -> Result<Option<String>, OAuthError> {
        let serve = async {
            loop {
                let stream = self.accept().await.map_err(OAuthError::Listen)?;
                if let Some(outcome) = self.serve(stream, state).await {
                    return outcome;
                }
            }
        };
        tokio::select! {
            outcome = serve => outcome.map(Some),
            () = cancel.cancelled() => Ok(None),
            () = tokio::time::sleep(timeout) => Err(OAuthError::TimedOut),
        }
    }

    /// The next connection on either address.
    async fn accept(&self) -> io::Result<TcpStream> {
        let ipv6 = async {
            match &self.ipv6 {
                Some(listener) => listener.accept().await,
                None => std::future::pending().await,
            }
        };
        let (stream, _): (TcpStream, SocketAddr) = tokio::select! {
            accepted = self.ipv4.accept() => accepted?,
            accepted = ipv6 => accepted?,
        };
        Ok(stream)
    }

    /// Answers one connection; `Some` once the sign-in has an outcome.
    async fn serve(
        &self,
        mut stream: TcpStream,
        state: &str,
    ) -> Option<Result<String, OAuthError>> {
        let head = tokio::time::timeout(READ_TIMEOUT, read_head(&mut stream))
            .await
            .ok()?
            .ok()?;
        let (page, outcome) = match callback::parse(&head, self.path) {
            Callback::Code {
                code,
                state: echoed,
            } if echoed == state => (Page::Done, Some(Ok(code))),
            Callback::Error {
                error,
                state: Some(echoed),
            } if echoed == state => (Page::Failed, Some(Err(provider_error(&error)))),
            Callback::Other => (Page::NotFound, None),
            Callback::Code { .. } | Callback::Error { .. } => (Page::Failed, None),
        };
        let reply = callback::response(page);
        // The outcome stands even if the browser hung up before the page.
        if let Err(error) = stream.write_all(reply.as_bytes()).await {
            log::debug!(target: "integrations", "the browser closed the redirect early: {error}");
        }
        outcome
    }
}

/// Reads up to the end of the request head, at most [`MAX_HEAD_BYTES`].
async fn read_head(stream: &mut TcpStream) -> io::Result<Vec<u8>> {
    let mut head = Vec::with_capacity(1024);
    let mut chunk = [0_u8; 1024];
    while !callback::is_complete(&head) && head.len() < MAX_HEAD_BYTES {
        let read = stream.read(&mut chunk).await?;
        if read == 0 {
            break;
        }
        head.extend_from_slice(chunk.get(..read).unwrap_or_default());
    }
    Ok(head)
}

/// A provider's redirect error as an [`OAuthError`].
fn provider_error(error: &str) -> OAuthError {
    match error {
        "access_denied" => OAuthError::AccessDenied,
        other => OAuthError::Provider(error_code(other)),
    }
}

/// A bind failure: a taken fixed port says so.
fn bind_error(port: u16, error: io::Error) -> OAuthError {
    if port != 0 && error.kind() == io::ErrorKind::AddrInUse {
        OAuthError::PortInUse(port)
    } else {
        OAuthError::Listen(error)
    }
}

#[cfg(test)]
mod loopback_tests;
