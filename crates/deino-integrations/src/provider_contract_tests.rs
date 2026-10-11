//! The [`Integration`] contract, run against every provider (§3A Liskov).
//! A provider card adds its provider to [`providers`].

use std::sync::Arc;

use deino_domain::{Capability, IntegrationProvider};
use deino_net::{FakeNetClient, FakeReply};
use deino_secrets::SecretValue;

use crate::{
    AuthSpec, Integration, ProviderApi, SandboxIntegration, ScriptedIntegration, TokenExtras,
    TokenScheme, TokenSpec,
};

/// Every provider, with the answer its `identify` needs from the network.
fn providers() -> Vec<(Arc<dyn Integration>, Option<&'static str>)> {
    let token = TokenSpec {
        label: "API key",
        username_label: None,
        scheme: TokenScheme::Bearer,
        help_url: "https://example.invalid".into(),
    };
    let scripted = ScriptedIntegration::new(
        IntegrationProvider::Linear,
        vec![AuthSpec::Token(token)],
        "https://api.linear.app",
    );
    vec![
        (Arc::new(SandboxIntegration::new()), None),
        (Arc::new(scripted), Some(r#"{"id":"org-1","name":"Acme"}"#)),
    ]
}

#[test]
fn capabilities_list_exactly_the_offered_accessors() {
    for (provider, _) in providers() {
        let offered = |capability| provider.capabilities().contains(&capability);
        assert_eq!(
            offered(Capability::PostMessage),
            provider.messages().is_some()
        );
        assert_eq!(offered(Capability::CreatePage), provider.pages().is_some());
        assert_eq!(offered(Capability::CreateTask), provider.tasks().is_some());
        assert_eq!(
            offered(Capability::LogActivity),
            provider.activities().is_some()
        );
        assert_eq!(
            offered(Capability::ReadContext),
            provider.context().is_some()
        );
    }
}

#[test]
fn every_provider_can_be_connected_and_paced() {
    for (provider, _) in providers() {
        let descriptor = provider.descriptor();
        assert_eq!(provider.id(), descriptor.id);
        assert!(
            !descriptor.auth.is_empty(),
            "{} has no way to connect",
            descriptor.id
        );
        assert!(descriptor.rate_limit.requests > 0 && !descriptor.rate_limit.per.is_zero());
    }
}

#[tokio::test]
async fn the_same_credential_names_the_same_account() {
    for (provider, answer) in providers() {
        let fake = Arc::new(FakeNetClient::default());
        for _ in 0..2 {
            fake.reply(FakeReply::ok(answer.unwrap_or_default()));
        }
        let token = SecretValue::parse("Bearer t").unwrap();
        let api = ProviderApi::new(fake, Some(token));

        let first = provider
            .identify(&api, &TokenExtras::default())
            .await
            .unwrap();
        let second = provider
            .identify(&api, &TokenExtras::default())
            .await
            .unwrap();

        assert_eq!(first, second);
        assert!(!first.id.is_empty() && !first.label.is_empty());
    }
}
