//! [`NetClient`]: the one interface every outgoing request goes through.
//!
//! The shape follows Zed's `HttpClient` (reference only): one required
//! method, [`NetClient::send`], and helpers built on it as provided methods,
//! so every implementation (the real client, the per-service
//! [`ScopedClient`](crate::ScopedClient), the test fake) gets them for free.
//! Generic JSON helpers live on [`NetClientExt`], implemented for every
//! client, because generic methods would stop `NetClient` from being used as
//! `dyn NetClient`.

use async_trait::async_trait;
use bytes::Bytes;
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::{MAX_BODY_BYTES, NetError, Request, Response, SseStream, sse};

/// Sends HTTP requests.
#[async_trait]
pub trait NetClient: Send + Sync {
    /// Sends `request` and returns as soon as the status and headers arrive;
    /// the body follows as a stream. Any status is a response, not an error.
    async fn send(&self, request: Request) -> Result<Response, NetError>;

    /// Sends `request` and reads the whole body of a 2xx response, at most
    /// [`MAX_BODY_BYTES`].
    async fn fetch(&self, request: Request) -> Result<Bytes, NetError> {
        let response = self.send(request).await?.error_for_status()?;
        response.bytes(MAX_BODY_BYTES).await
    }

    /// Sends `request` and decodes the Server-Sent Events of a 2xx response
    /// (LLM token streams).
    async fn events(&self, request: Request) -> Result<SseStream, NetError> {
        let request = request.header("accept", "text/event-stream");
        let response = self.send(request).await?.error_for_status()?;
        Ok(sse::events(response.body))
    }
}

/// JSON helpers for every [`NetClient`], including `dyn NetClient`.
#[async_trait]
pub trait NetClientExt: NetClient {
    /// `GET`s `url` and decodes a 2xx JSON body.
    async fn get_json<T: DeserializeOwned>(&self, url: &str) -> Result<T, NetError> {
        let request = Request::get(url)?.header("accept", "application/json");
        self.send(request).await?.error_for_status()?.json().await
    }

    /// Sends `request` with `body` as JSON and decodes a 2xx JSON answer.
    async fn exchange_json<B, T>(&self, request: Request, body: &B) -> Result<T, NetError>
    where
        B: Serialize + Sync,
        T: DeserializeOwned,
    {
        let request = request.json(body)?.header("accept", "application/json");
        self.send(request).await?.error_for_status()?.json().await
    }
}

impl<C: NetClient + ?Sized> NetClientExt for C {}

#[cfg(test)]
mod tests {
    use futures_util::StreamExt;
    use serde::Deserialize;

    use super::*;
    use crate::{FakeNetClient, FakeReply, Status};

    #[derive(Debug, Deserialize, PartialEq)]
    struct Models {
        data: Vec<String>,
    }

    #[tokio::test]
    async fn json_helpers_decode_a_success_and_reject_an_error_status() {
        let fake = FakeNetClient::default();
        fake.reply(FakeReply::ok(r#"{ "data": ["gpt"] }"#));
        fake.reply(FakeReply::status(Status::NOT_FOUND));
        let client: &dyn NetClient = &fake;

        let models: Models = client
            .get_json("https://api.openai.com/v1/models")
            .await
            .unwrap();
        let missing = client
            .get_json::<Models>("https://api.openai.com/v1/models")
            .await;

        assert_eq!(models.data, ["gpt"]);
        assert!(matches!(missing, Err(NetError::Status(Status::NOT_FOUND))));
    }

    #[tokio::test]
    async fn events_ask_for_a_stream_and_decode_it() {
        let fake = FakeNetClient::default();
        fake.reply(FakeReply::ok("data: one\n\ndata: two\n\n"));

        let request = Request::post("https://api.anthropic.com/v1/messages").unwrap();
        let events: Vec<_> = fake.events(request).await.unwrap().collect().await;

        let data: Vec<String> = events
            .into_iter()
            .map(|event| event.unwrap().data)
            .collect();
        assert_eq!(data, ["one", "two"]);
        let sent = fake.requests();
        assert_eq!(sent[0].headers.get("accept"), Some("text/event-stream"));
    }
}
