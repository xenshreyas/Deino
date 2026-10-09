//! The gateway's [`HttpTransport`] over `deino-net` (FND-12).
//!
//! Each provider gets its own [`NetTransport`] around a `ScopedClient` for
//! its `ServiceId`, so Anthropic's adapter can reach only Anthropic's hosts,
//! a switched-off provider sends nothing, offline mode stops every call, and
//! each request lands in the egress log. Headers and bodies pass through
//! untouched and are never logged.

use async_trait::async_trait;
use deino_domain::ServiceId;
use deino_llm::{Flavor, HttpError, HttpRequest, HttpResponse, HttpTransport, Method};
use deino_net::{Net, NetClient, NetError, Request, ScopedClient};
use futures_util::{StreamExt, TryStreamExt};

/// The service an OpenAI-compatible provider's requests belong to.
pub(crate) const fn flavor_service(flavor: Flavor) -> ServiceId {
    match flavor {
        Flavor::OpenAi => ServiceId::LlmOpenAi,
        Flavor::Groq => ServiceId::LlmGroq,
        Flavor::OpenRouter => ServiceId::LlmOpenRouter,
        Flavor::Custom => ServiceId::LlmCustom,
        Flavor::Ollama => ServiceId::LlmOllama,
    }
}

/// Sends one provider's requests through its scoped client.
#[derive(Clone)]
pub(crate) struct NetTransport {
    client: ScopedClient,
}

impl NetTransport {
    /// A transport confined to `service`.
    pub(crate) fn new(net: &Net, service: ServiceId) -> Self {
        Self {
            client: net.scoped(service),
        }
    }
}

#[async_trait]
impl HttpTransport for NetTransport {
    async fn send(&self, request: HttpRequest) -> Result<HttpResponse, HttpError> {
        let response = self.client.send(to_net(request)?).await.map_err(to_http)?;
        Ok(HttpResponse {
            status: response.status.as_u16(),
            headers: response
                .headers
                .iter()
                .map(|(name, value)| (name.to_owned(), value.to_owned()))
                .collect(),
            body: response
                .body
                .map_ok(|chunk| chunk.to_vec())
                .map_err(to_http)
                .boxed(),
        })
    }
}

/// The gateway's request as a `deino-net` request.
fn to_net(request: HttpRequest) -> Result<Request, HttpError> {
    let method = match request.method {
        Method::Get => deino_net::Method::Get,
        Method::Post => deino_net::Method::Post,
        Method::Delete => deino_net::Method::Delete,
    };
    let mut net = Request::new(method, &request.url)
        .map_err(to_http)?
        .timeout(Some(request.timeout));
    for (name, value) in request.headers {
        net = net.header(name, value);
    }
    if let Some(body) = request.body {
        net = net.body(body);
    }
    Ok(net)
}

/// A `deino-net` failure in the gateway's terms. Its message is user-safe:
/// a blocked request says why (offline mode, a switched-off service).
fn to_http(error: NetError) -> HttpError {
    match error {
        NetError::Timeout(_) => HttpError::Timeout,
        NetError::Transport(_) => HttpError::Transport(error.to_string()),
        other => HttpError::Connect(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use chrono::{TimeZone, Utc};
    use deino_domain::FakeClock;
    use deino_net::{EgressLog, FakeNetClient, FakeReply, FakeServicePolicy, NetParts};

    use super::*;

    fn net(wire: Arc<FakeNetClient>, policy: Arc<FakeServicePolicy>) -> Net {
        let clock = FakeClock::new(Utc.with_ymd_and_hms(2026, 10, 9, 9, 0, 0).unwrap());
        Net::new(NetParts {
            client: wire,
            policy,
            egress: Arc::new(EgressLog::default()),
            clock: Arc::new(clock),
        })
    }

    fn messages_request() -> HttpRequest {
        HttpRequest {
            method: Method::Post,
            url: "https://api.anthropic.com/v1/messages".into(),
            headers: vec![("x-api-key", "sk-ant".into())],
            body: Some(b"{}".to_vec()),
            timeout: Duration::from_secs(5),
        }
    }

    #[tokio::test]
    async fn a_provider_request_passes_through_with_its_headers_and_body() {
        let wire = Arc::new(FakeNetClient::default());
        wire.reply(FakeReply::ok("{\"ok\":true}").header("content-type", "application/json"));
        let transport =
            NetTransport::new(&net(wire.clone(), Arc::default()), ServiceId::LlmAnthropic);

        let response = transport.send(messages_request()).await.unwrap();

        assert_eq!(response.status, 200);
        assert_eq!(response.header("Content-Type"), Some("application/json"));
        assert_eq!(response.bytes().await.unwrap(), b"{\"ok\":true}");
        let sent = &wire.requests()[0];
        assert_eq!(sent.headers.get("x-api-key"), Some("sk-ant"));
        assert_eq!(sent.body.as_deref(), Some(b"{}".as_slice()));
    }

    #[tokio::test]
    async fn a_provider_cannot_reach_another_providers_host_or_send_offline() {
        let wire = Arc::new(FakeNetClient::default());
        let policy = Arc::new(FakeServicePolicy::default());
        let net = net(wire.clone(), policy.clone());
        let openai = NetTransport::new(&net, ServiceId::LlmOpenAi);

        let elsewhere = openai.send(messages_request()).await;
        policy.set_offline(true);
        let offline = NetTransport::new(&net, ServiceId::LlmAnthropic)
            .send(messages_request())
            .await;

        assert!(matches!(elsewhere, Err(HttpError::Connect(_))));
        let Err(HttpError::Connect(message)) = offline else {
            panic!("offline mode must refuse the request");
        };
        assert!(message.contains("offline mode is on"));
        assert!(wire.requests().is_empty());
    }

    #[test]
    fn every_openai_compatible_provider_has_its_own_service() {
        assert_eq!(flavor_service(Flavor::Groq), ServiceId::LlmGroq);
        assert_eq!(flavor_service(Flavor::Custom), ServiceId::LlmCustom);
    }
}
