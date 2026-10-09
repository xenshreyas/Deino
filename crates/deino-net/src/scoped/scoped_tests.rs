//! What a scoped client lets out, and what the egress log says about it.

use std::sync::Arc;

use chrono::{TimeZone, Utc};
use deino_domain::{FakeClock, ServiceId};
use futures_util::StreamExt;

use super::*;
use crate::egress::EgressReader;
use crate::{
    EgressEntry, FakeNetClient, FakeReply, FakeServicePolicy, Method, ServiceSwitch, Status,
};

struct Harness {
    net: Net,
    wire: Arc<FakeNetClient>,
    policy: Arc<FakeServicePolicy>,
    egress: Arc<EgressLog>,
}

fn harness() -> Harness {
    let wire = Arc::new(FakeNetClient::default());
    let policy = Arc::new(FakeServicePolicy::default());
    let egress = Arc::new(EgressLog::default());
    let clock = FakeClock::new(Utc.with_ymd_and_hms(2026, 10, 9, 9, 0, 0).unwrap());
    let net = Net::new(NetParts {
        client: wire.clone(),
        policy: policy.clone(),
        egress: egress.clone(),
        clock: Arc::new(clock),
    });
    Harness {
        net,
        wire,
        policy,
        egress,
    }
}

impl Harness {
    async fn log(&self) -> Vec<EgressEntry> {
        self.egress.recent(100).await.unwrap()
    }
}

fn get(url: &str) -> Request {
    Request::get(url).unwrap()
}

#[tokio::test]
async fn a_service_reaches_its_own_hosts_and_the_exchange_is_logged() {
    let h = harness();
    h.wire.reply(FakeReply::ok("{\"id\":\"msg\"}"));
    let request = Request::post("https://api.anthropic.com/v1/messages")
        .unwrap()
        .body("12345");

    let body = h.net.scoped(ServiceId::LlmAnthropic).fetch(request).await.unwrap();

    assert_eq!(body.len(), 12);
    let log = h.log().await;
    assert_eq!(log.len(), 1);
    assert_eq!(log[0].host, "api.anthropic.com");
    assert_eq!(log[0].method, Method::Post);
    assert_eq!(log[0].status, Some(200));
    assert_eq!(log[0].outcome, EgressOutcome::Sent);
    assert_eq!((log[0].bytes_up, log[0].bytes_down), (5, 12));
}

#[tokio::test]
async fn an_unregistered_host_is_refused_before_anything_is_sent() {
    let h = harness();

    let result = h
        .net
        .scoped(ServiceId::LlmAnthropic)
        .send(get("https://api.openai.com/v1/models"))
        .await;

    assert!(matches!(result, Err(NetError::HostNotAllowed { .. })));
    assert!(h.wire.requests().is_empty());
    let log = h.log().await;
    assert_eq!(log[0].outcome, EgressOutcome::Blocked);
    assert_eq!(log[0].status, None);
}

#[tokio::test]
async fn a_disabled_service_sends_nothing_until_it_is_switched_on() {
    let h = harness();
    let slack = h.net.scoped(ServiceId::Slack);

    let off = slack.send(get("https://slack.com/api/chat.postMessage")).await;
    h.policy.set_enabled(ServiceId::Slack, true).await.unwrap();
    let on = slack.send(get("https://slack.com/api/chat.postMessage")).await;

    assert!(matches!(off, Err(NetError::ServiceDisabled(ServiceId::Slack))));
    assert!(on.is_ok());
    assert_eq!(h.wire.requests().len(), 1);
}

#[tokio::test]
async fn a_user_switch_overrides_a_default_on_service() {
    let h = harness();
    h.policy
        .set_enabled(ServiceId::LlmOpenAi, false)
        .await
        .unwrap();

    let result = h
        .net
        .scoped(ServiceId::LlmOpenAi)
        .send(get("https://api.openai.com/v1/models"))
        .await;

    assert!(matches!(result, Err(NetError::ServiceDisabled(_))));
}

#[tokio::test]
async fn a_custom_endpoint_is_reachable_only_by_the_service_that_takes_it() {
    let h = harness();
    h.policy.add_custom_host(ServiceId::LlmCustom, "llm.internal.test");
    h.policy.add_custom_host(ServiceId::LlmAnthropic, "llm.internal.test");

    let custom = h
        .net
        .scoped(ServiceId::LlmCustom)
        .send(get("http://llm.internal.test:8000/v1/chat/completions"))
        .await;
    let anthropic = h
        .net
        .scoped(ServiceId::LlmAnthropic)
        .send(get("http://llm.internal.test:8000/v1/chat/completions"))
        .await;

    assert!(matches!(anthropic, Err(NetError::HostNotAllowed { .. })));
    drop(custom.unwrap());
    let log = h.log().await;
    assert_eq!(log.len(), 2);
    assert!(log.iter().all(|entry| entry.host == "llm.internal.test:8000"));
}

#[tokio::test]
async fn offline_mode_blocks_every_service_but_approved_model_downloads() {
    let h = harness();
    h.policy.set_offline(true);

    for &service in &[ServiceId::LlmAnthropic, ServiceId::LlmOllama, ServiceId::HfModels] {
        let url = match service {
            ServiceId::LlmAnthropic => "https://api.anthropic.com/v1/messages",
            ServiceId::LlmOllama => "http://localhost:11434/api/tags",
            _ => "https://huggingface.co/m/resolve/main/model.gguf",
        };
        let result = h.net.scoped(service).send(get(url)).await;
        assert!(matches!(result, Err(NetError::Offline(_))), "{service}");
    }
    let download = h.net.approved_download(ServiceId::HfModels).unwrap();
    let downloaded = download
        .send(get("https://huggingface.co/m/resolve/main/model.gguf"))
        .await;

    assert!(downloaded.is_ok());
    assert_eq!(h.wire.requests().len(), 1);
    assert!(matches!(
        h.net.approved_download(ServiceId::LlmAnthropic),
        Err(NetError::NotADownloadService(_))
    ));
}

#[tokio::test]
async fn a_switched_off_service_blocks_even_approved_downloads() {
    let h = harness();
    h.policy
        .set_enabled(ServiceId::HfModels, false)
        .await
        .unwrap();
    let download = h.net.approved_download(ServiceId::HfModels).unwrap();

    let result = download.send(get("https://huggingface.co/m.gguf")).await;

    assert!(matches!(result, Err(NetError::ServiceDisabled(_))));
}

#[tokio::test]
async fn redirects_are_followed_within_the_service_and_each_hop_is_logged() {
    let h = harness();
    h.wire.reply(FakeReply::redirect(
        Status::FOUND,
        "https://cas-bridge.xethub.hf.co/blob?X-Amz-Signature=secret",
    ));
    h.wire.reply(FakeReply::ok("weights"));
    let request = get("https://huggingface.co/m/resolve/main/model.gguf")
        .header("authorization", "Bearer hf_token");

    let body = h.net.scoped(ServiceId::HfModels).fetch(request).await.unwrap();

    assert_eq!(&body[..], b"weights");
    let sent = h.wire.requests();
    assert_eq!(sent[1].headers.get("authorization"), None);
    let hosts: Vec<String> = h.log().await.into_iter().map(|entry| entry.host).collect();
    assert_eq!(hosts, ["cas-bridge.xethub.hf.co", "huggingface.co"]);
}

#[tokio::test]
async fn a_redirect_off_the_service_is_refused() {
    let h = harness();
    h.wire.reply(FakeReply::redirect(Status::FOUND, "https://evil.test/steal"));

    let result = h
        .net
        .scoped(ServiceId::LlmAnthropic)
        .send(get("https://api.anthropic.com/v1/messages"))
        .await;

    assert!(matches!(result, Err(NetError::HostNotAllowed { .. })));
    assert_eq!(h.wire.requests().len(), 1);
}

#[tokio::test]
async fn the_egress_log_never_holds_a_path_a_query_a_header_or_a_body() {
    let h = harness();
    h.wire.reply(FakeReply::ok("the model said something private"));
    let request = Request::post("https://api.openai.com/v1/chat/completions?api-key=sk-query")
        .unwrap()
        .header("authorization", "Bearer sk-header")
        .body("the meeting transcript");

    let response = h.net.scoped(ServiceId::LlmOpenAi).send(request).await.unwrap();
    let _received: Vec<_> = response.body.collect().await;

    let entries = h.log().await;
    let logged = format!("{entries:?} {}", serde_json::to_string(&entries).unwrap());
    for secret in [
        "/v1/chat",
        "api-key",
        "sk-query",
        "sk-header",
        "transcript",
        "private",
    ] {
        assert!(!logged.contains(secret), "the egress log leaks {secret}: {logged}");
    }
    assert!(logged.contains("api.openai.com"));
}
