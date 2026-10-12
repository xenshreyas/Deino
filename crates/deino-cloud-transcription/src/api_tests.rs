//! The two calls over recorded answers (`fixtures/elevenlabs/`): what is
//! sent, what comes back, and every documented failure. Never the network.

use deino_domain::{FakeClock, ServiceId};
use deino_net::{
    EgressLog, EgressOutcome, EgressReader, FakeNetClient, FakeReply, FakeServicePolicy, Net,
    NetParts, Status,
};

use super::*;
use crate::wire::ScribeWordKind;

fn api(client: &Arc<FakeNetClient>) -> ScribeApi {
    let client: Arc<dyn NetClient> = client.clone();
    ScribeApi::new(client, "xi-secret".into())
}

fn answer(status: u16, body: &'static str) -> FakeReply {
    FakeReply::ok(body).with_status(Status::new(status))
}

const OPTIONS: ScribeOptions<'static> = ScribeOptions {
    model: "scribe_v2",
    diarize: true,
};

#[tokio::test]
async fn a_track_is_sent_as_a_multipart_upload_with_the_key() {
    let client = Arc::new(FakeNetClient::default());
    client.reply(FakeReply::ok(include_str!(
        "../fixtures/elevenlabs/convert-system-diarized.json"
    )));

    let transcript = api(&client)
        .transcribe(("system.m4a", b"AAC"), OPTIONS)
        .await
        .unwrap();

    assert_eq!(transcript.language_code, "en");
    assert_eq!(transcript.words.len(), 22);
    assert_eq!(transcript.words[13].kind, ScribeWordKind::AudioEvent);
    let request = &client.requests()[0];
    assert_eq!(request.url.as_str(), CONVERT_URL);
    assert_eq!(request.headers.get("xi-api-key"), Some("xi-secret"));
    let body = String::from_utf8(request.body.clone().unwrap().to_vec()).unwrap();
    for field in [
        "name=\"model_id\"\r\n\r\nscribe_v2\r\n",
        "name=\"language_code\"\r\n\r\nen\r\n",
        "name=\"diarize\"\r\n\r\ntrue\r\n",
        "name=\"timestamps_granularity\"\r\n\r\nword\r\n",
        "name=\"tag_audio_events\"\r\n\r\nfalse\r\n",
        "filename=\"system.m4a\"\r\nContent-Type: audio/mp4\r\n\r\nAAC\r\n",
    ] {
        assert!(body.contains(field), "{field:?} missing");
    }
    let content_type = request.headers.get("content-type").unwrap();
    assert!(content_type.starts_with("multipart/form-data; boundary=deino-"));
}

/// The error a transcription answered with `status` and `fixtures/elevenlabs/<file>` becomes.
async fn failure(status: u16, file: &str) -> CloudError {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures/elevenlabs")
        .join(file);
    let client = Arc::new(FakeNetClient::default());
    client.reply(FakeReply::ok(std::fs::read(path).unwrap()).with_status(Status::new(status)));
    let error = api(&client)
        .transcribe(("mic.m4a", b"AAC"), OPTIONS)
        .await
        .unwrap_err();
    assert!(!error.to_string().contains("xi-secret"));
    error
}

#[tokio::test]
async fn documented_failures_become_typed_errors() {
    let refused = failure(401, "error-401-invalid-key.json").await;
    let credits = failure(402, "error-402-credits.json").await;
    let permissions = failure(403, "error-403-permissions.json").await;
    let too_large = failure(400, "error-400-too-large.json").await;
    let model = failure(422, "error-422-model.json").await;
    let busy = failure(429, "error-429-concurrency.json").await;
    let down = failure(503, "error-503-busy.json").await;

    assert!(matches!(refused, CloudError::KeyRefused), "{refused:?}");
    assert!(matches!(credits, CloudError::NoCredits), "{credits:?}");
    assert!(
        matches!(permissions, CloudError::NotPermitted),
        "{permissions:?}"
    );
    assert!(matches!(too_large, CloudError::TooLarge), "{too_large:?}");
    assert!(matches!(model, CloudError::Rejected), "{model:?}");
    assert!(matches!(busy, CloudError::RateLimited), "{busy:?}");
    assert!(matches!(down, CloudError::Unavailable), "{down:?}");
}

#[tokio::test]
async fn an_answer_that_is_not_the_documented_json_is_unreadable() {
    let client = Arc::new(FakeNetClient::default());
    client.reply(FakeReply::ok("<html>maintenance</html>"));

    let error = api(&client).transcribe(("mic.m4a", b"AAC"), OPTIONS).await;

    assert!(matches!(error, Err(CloudError::Unreadable)));
}

#[tokio::test]
async fn the_check_accepts_a_known_key_even_when_it_is_restricted() {
    let client = Arc::new(FakeNetClient::default());
    client.reply(FakeReply::ok(include_str!(
        "../fixtures/elevenlabs/user.json"
    )));
    client.reply(answer(
        403,
        include_str!("../fixtures/elevenlabs/error-403-permissions.json"),
    ));
    client.reply(answer(
        401,
        include_str!("../fixtures/elevenlabs/error-401-invalid-key.json"),
    ));

    assert!(api(&client).check().await.is_ok());
    assert!(api(&client).check().await.is_ok());
    assert!(matches!(
        api(&client).check().await,
        Err(CloudError::KeyRefused)
    ));
    let requests = client.requests();
    assert_eq!(requests[0].url.as_str(), USER_URL);
    assert_eq!(requests[0].headers.get("xi-api-key"), Some("xi-secret"));
}

#[tokio::test]
async fn a_switched_off_service_sends_nothing_and_the_egress_log_says_so() {
    let client = Arc::new(FakeNetClient::default());
    let egress = Arc::new(EgressLog::default());
    let net = Net::new(NetParts {
        client: client.clone(),
        policy: Arc::new(FakeServicePolicy::default()),
        egress: Arc::clone(&egress),
        clock: Arc::new(FakeClock::new(chrono::DateTime::UNIX_EPOCH)),
    });
    let scoped: Arc<dyn NetClient> = Arc::new(net.scoped(ServiceId::ElevenLabs));

    let error = ScribeApi::new(scoped, "xi-secret".into()).check().await;

    assert!(matches!(error, Err(CloudError::Blocked)));
    assert!(client.requests().is_empty());
    let logged = egress.recent(10).await.unwrap();
    assert_eq!(logged[0].service, ServiceId::ElevenLabs);
    assert_eq!(logged[0].outcome, EgressOutcome::Blocked);
    assert_eq!(logged[0].host, API_HOST);
}
