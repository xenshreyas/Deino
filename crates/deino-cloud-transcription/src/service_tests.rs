//! The key's life cycle and the choice per meeting, over fakes.

use deino_net::{FakeNetClient, FakeReply, FakeServicePolicy, ServicePolicy};
use deino_secrets::InMemorySecretStore;

use super::*;

/// The cloud option fixed for a test.
struct Fixed(Mutex<CloudTranscriptionProvider>);

#[async_trait]
impl CloudSettings for Fixed {
    async fn cloud(&self) -> (CloudTranscriptionProvider, CloudTranscriptionModel) {
        (*lock(&self.0), CloudTranscriptionModel::ScribeV2)
    }
}

struct World {
    client: Arc<FakeNetClient>,
    policy: Arc<FakeServicePolicy>,
    settings: Arc<Fixed>,
    service: ElevenLabs,
}

fn world() -> World {
    let client = Arc::new(FakeNetClient::default());
    let policy = Arc::new(FakeServicePolicy::default());
    let settings = Arc::new(Fixed(Mutex::new(CloudTranscriptionProvider::None)));
    let service = ElevenLabs::new(ElevenLabsParts {
        client: client.clone(),
        secrets: Arc::new(InMemorySecretStore::default()),
        switch: policy.clone(),
        settings: settings.clone(),
    });
    World {
        client,
        policy,
        settings,
        service,
    }
}

impl World {
    async fn allowed(&self) -> bool {
        self.policy.grant(ServiceId::ElevenLabs).await.enabled
    }
}

#[tokio::test]
async fn saving_a_key_switches_the_service_on_and_forgetting_it_off() {
    let world = world();
    assert!(!world.allowed().await, "off until the user sets it up");

    world.service.save_key("xi-secret").await.unwrap();
    assert_eq!(
        world.service.key_status().await.unwrap(),
        SecretStatus::Stored
    );
    assert!(world.allowed().await);

    world.service.forget_key().await.unwrap();
    assert_eq!(
        world.service.key_status().await.unwrap(),
        SecretStatus::Missing
    );
    assert!(!world.allowed().await);
    assert!(world.service.save_key("  ").await.is_err());
}

#[tokio::test]
async fn checking_asks_elevenlabs_with_the_saved_key() {
    let world = world();
    let unsaved = world.service.check_key().await;
    world.service.save_key("xi-secret").await.unwrap();
    world.client.reply(FakeReply::ok(include_str!(
        "../fixtures/elevenlabs/user.json"
    )));

    world.service.check_key().await.unwrap();

    assert!(matches!(
        unsaved,
        Err(KeyError::Cloud(CloudError::MissingKey))
    ));
    let requests = world.client.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].headers.get("xi-api-key"), Some("xi-secret"));
}

#[tokio::test]
async fn a_meeting_goes_to_the_cloud_when_chosen_with_a_key_or_asked_for() {
    let world = world();
    let meeting = MeetingId::new("meeting-a");
    *lock(&world.settings.0) = CloudTranscriptionProvider::ElevenLabs;
    assert!(
        world.service.for_meeting(&meeting).await.is_none(),
        "no key yet"
    );
    assert!(world.service.request(&meeting).await.is_err(), "no key yet");

    world.service.save_key("xi-secret").await.unwrap();
    let chosen = world.service.for_meeting(&meeting).await.unwrap();
    assert!(!chosen.requested);

    *lock(&world.settings.0) = CloudTranscriptionProvider::None;
    assert!(world.service.for_meeting(&meeting).await.is_none());
    world.service.request(&meeting).await.unwrap();
    let asked = world.service.for_meeting(&meeting).await.unwrap();
    assert!(asked.requested);

    world.service.finished(&meeting);
    assert!(world.service.for_meeting(&meeting).await.is_none());
}
