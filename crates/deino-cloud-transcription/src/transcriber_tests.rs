//! One track end to end over a recorded answer: the file on disk, the key in
//! a fake Keychain, the lines that come back.

use deino_domain::Millis;
use deino_net::{FakeNetClient, FakeReply};
use deino_secrets::{InMemorySecretStore, SecretValue};
use tokio_util::sync::CancellationToken;

use super::*;

struct World {
    client: Arc<FakeNetClient>,
    secrets: Arc<InMemorySecretStore>,
    folder: tempfile::TempDir,
}

impl World {
    async fn new() -> Self {
        let folder = tempfile::tempdir().unwrap();
        let track = folder.path().join("system.m4a");
        tokio::fs::write(track, b"AAC").await.unwrap();
        let secrets = Arc::new(InMemorySecretStore::default());
        let key = SecretValue::parse("xi-secret").unwrap();
        secrets
            .set(SecretName::ElevenLabsApiKey, key)
            .await
            .unwrap();
        Self {
            client: Arc::new(FakeNetClient::default()),
            secrets,
            folder,
        }
    }

    fn transcriber(&self) -> ElevenLabsTranscriber {
        let client: Arc<dyn NetClient> = self.client.clone();
        let secrets: Arc<dyn SecretStore> = self.secrets.clone();
        ElevenLabsTranscriber::new(client, secrets, CloudTranscriptionModel::ScribeV1)
    }

    fn request(&self, file: &str) -> CloudRequest {
        CloudRequest {
            audio: self.folder.path().join(file),
            diarize: true,
            cancel: CancellationToken::new(),
        }
    }
}

#[tokio::test]
async fn a_diarized_track_comes_back_as_lines_with_voices() {
    let world = World::new().await;
    world.client.reply(FakeReply::ok(include_str!(
        "../fixtures/elevenlabs/convert-system-diarized.json"
    )));

    let lines = world
        .transcriber()
        .transcribe(world.request("system.m4a"))
        .await
        .unwrap();

    let texts: Vec<(&str, Option<&str>)> = lines
        .iter()
        .map(|line| (line.span.text.as_str(), line.voice.as_deref()))
        .collect();
    assert_eq!(
        texts,
        [
            ("Morning everyone.", Some("speaker_0")),
            ("Hey, can you hear me?", Some("speaker_1")),
            ("Yes, loud and clear.", Some("speaker_0")),
        ]
    );
    assert_eq!(lines[0].span.start, Millis(420));
    let body = world.client.requests()[0].body.clone().unwrap();
    let body = String::from_utf8(body.to_vec()).unwrap();
    assert!(body.contains("name=\"model_id\"\r\n\r\nscribe_v1\r\n"));
}

#[tokio::test]
async fn without_a_key_or_a_file_nothing_is_sent() {
    let world = World::new().await;
    let gone = world
        .transcriber()
        .transcribe(world.request("gone.m4a"))
        .await;
    world
        .secrets
        .delete(SecretName::ElevenLabsApiKey)
        .await
        .unwrap();
    let keyless = world
        .transcriber()
        .transcribe(world.request("system.m4a"))
        .await;

    assert!(matches!(gone, Err(CloudError::Audio(_))));
    assert!(matches!(keyless, Err(CloudError::MissingKey)));
    assert!(world.client.requests().is_empty());
}

#[tokio::test]
async fn a_cancelled_run_stops() {
    let world = World::new().await;
    let request = world.request("system.m4a");
    request.cancel.cancel();

    let cancelled = world.transcriber().transcribe(request).await;

    assert!(matches!(cancelled, Err(CloudError::Cancelled)));
}

#[test]
fn the_upload_is_named_after_the_track_only() {
    assert_eq!(file_name(Path::new("/r/meeting/mic.m4a")), "mic.m4a");
    assert_eq!(file_name(Path::new("/")), "audio.m4a");
}
