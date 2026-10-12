//! [`ProviderApi`]: a provider's JSON API over its `deino-net` client, so
//! the user's switch, offline mode and the egress log apply (ADR 0019). It
//! adds the key, maps a refused key and rate limits, and reads the RFC 8288
//! `Link: <...>; rel="next"` header Circleback pages with.

use std::sync::Arc;
use std::time::Duration;

use deino_core::importers::ImporterError;
use deino_domain::ImportSource;
use deino_net::{MAX_BODY_BYTES, NetClient, Request, Response};
use serde::Serialize;
use serde::de::DeserializeOwned;

/// How many times a request is sent while the provider answers 429.
const ATTEMPTS: u32 = 3;
/// The longest `Retry-After` honoured before giving up on the run.
const MAX_RETRY_AFTER: Duration = Duration::from_secs(30);
/// The wait when a 429 names none.
const DEFAULT_RETRY_AFTER: Duration = Duration::from_secs(5);

/// A decoded answer and the next page's URL, if the server linked one.
pub(crate) struct Fetched<T> {
    /// The body.
    pub(crate) body: T,
    /// The `rel="next"` link.
    pub(crate) next: Option<String>,
}

/// One provider's API with the user's key.
pub(crate) struct ProviderApi {
    client: Arc<dyn NetClient>,
    source: ImportSource,
    auth_header: &'static str,
    auth_value: String,
}

impl ProviderApi {
    /// The API of `source` over `client`, sending `auth_value` in `auth_header`.
    pub(crate) fn new(
        client: Arc<dyn NetClient>,
        source: ImportSource,
        auth_header: &'static str,
        auth_value: String,
    ) -> Self {
        Self {
            client,
            source,
            auth_header,
            auth_value,
        }
    }

    /// `GET`s `url` and decodes its JSON.
    pub(crate) async fn get<T: DeserializeOwned>(
        &self,
        url: &str,
    ) -> Result<Fetched<T>, ImporterError> {
        let response = self.send(Request::get(url)?).await?;
        let next = response.header("link").and_then(next_link);
        let body = self.decode(response).await?;
        Ok(Fetched { body, next })
    }

    /// `POST`s `body` as JSON to `url` and decodes the answer.
    pub(crate) async fn post<B, T>(&self, url: &str, body: &B) -> Result<T, ImporterError>
    where
        B: Serialize + Sync,
        T: DeserializeOwned,
    {
        let request = Request::post(url)?.json(body)?;
        let response = self.send(request).await?;
        self.decode(response).await
    }

    /// Sends `request` with the key, waiting out up to [`ATTEMPTS`] 429s.
    async fn send(&self, request: Request) -> Result<Response, ImporterError> {
        let request = request
            .header(self.auth_header, self.auth_value.clone())
            .header("accept", "application/json");
        for _ in 0..ATTEMPTS {
            let response = self.client.send(request.clone()).await?;
            match response.status.as_u16() {
                401 | 403 => return Err(ImporterError::Unauthorized(self.source)),
                429 => tokio::time::sleep(retry_after(&response)).await,
                _ => return Ok(response.error_for_status()?),
            }
        }
        Err(ImporterError::RateLimited(self.source))
    }

    async fn decode<T: DeserializeOwned>(&self, response: Response) -> Result<T, ImporterError> {
        let bytes = response.bytes(MAX_BODY_BYTES).await?;
        serde_json::from_slice(&bytes).map_err(|error| {
            log::warn!(target: "importers", "{} answered JSON Deino cannot read: {error}", self.source);
            ImporterError::Unreadable(self.source, "unexpected JSON")
        })
    }
}

/// How long a 429 asks to wait, capped.
fn retry_after(response: &Response) -> Duration {
    response
        .header("retry-after")
        .and_then(|value| value.trim().parse::<u64>().ok())
        .map_or(DEFAULT_RETRY_AFTER, Duration::from_secs)
        .min(MAX_RETRY_AFTER)
}

/// The `rel="next"` target of an RFC 8288 `Link` header.
pub(crate) fn next_link(header: &str) -> Option<String> {
    header.split(',').find_map(|link| {
        let (target, params) = link.split_once(';')?;
        let is_next = params
            .split(';')
            .any(|param| param.trim().replace(' ', "") == "rel=\"next\"");
        let target = target.trim().strip_prefix('<')?.strip_suffix('>')?;
        is_next.then(|| target.to_owned())
    })
}

#[cfg(test)]
mod tests {
    use deino_net::{FakeNetClient, FakeReply, Status};

    use super::*;

    fn api(client: &Arc<FakeNetClient>) -> ProviderApi {
        let client: Arc<dyn NetClient> = client.clone();
        ProviderApi::new(client, ImportSource::Circleback, "authorization", "Bearer k".into())
    }

    #[test]
    fn the_next_link_is_found_among_others() {
        let header = r#"<https://a.test/x?cursor=1>; rel="prev", <https://a.test/x?cursor=2>; rel="next""#;
        assert_eq!(next_link(header).as_deref(), Some("https://a.test/x?cursor=2"));
        assert_eq!(next_link(r#"<https://a.test/x>; rel="prev""#), None);
    }

    #[tokio::test]
    async fn a_rate_limit_is_waited_out_and_the_key_is_sent() {
        let client = Arc::new(FakeNetClient::default());
        client.reply(FakeReply::status(Status::new(429)).header("retry-after", "0"));
        client.reply(FakeReply::ok("[1]"));

        let fetched: Fetched<Vec<u8>> = api(&client).get("https://circleback.ai/api/x").await.unwrap();

        assert_eq!(fetched.body, vec![1]);
        let requests = client.requests();
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[1].headers.get("authorization"), Some("Bearer k"));
    }

    #[tokio::test]
    async fn a_refused_key_and_endless_limits_are_named() {
        let client = Arc::new(FakeNetClient::default());
        client.reply(FakeReply::status(Status::new(401)));
        let refused = api(&client).get::<Vec<u8>>("https://circleback.ai/api/x").await;
        assert!(matches!(refused, Err(ImporterError::Unauthorized(_))));

        for _ in 0..ATTEMPTS {
            client.reply(FakeReply::status(Status::new(429)).header("retry-after", "0"));
        }
        let limited = api(&client).get::<Vec<u8>>("https://circleback.ai/api/x").await;
        assert!(matches!(limited, Err(ImporterError::RateLimited(_))));
    }
}
