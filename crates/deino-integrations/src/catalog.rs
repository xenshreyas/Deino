//! The planned providers: every `IntegrationProvider` this build does not ship yet,
//! with what it will do, so Settings → Integrations can list them as coming
//! soon. A provider card (INT-01..07) moves its provider into
//! [`crate::registry`]; nothing here changes.

use deino_domain::{Capability, IntegrationProvider};

/// What `provider` is planned to do once its card lands.
pub const fn planned_capabilities(provider: IntegrationProvider) -> &'static [Capability] {
    match provider {
        IntegrationProvider::Slack => &[Capability::PostMessage],
        IntegrationProvider::Notion | IntegrationProvider::GoogleDocs => &[Capability::CreatePage],
        IntegrationProvider::Linear
        | IntegrationProvider::Todoist
        | IntegrationProvider::Asana
        | IntegrationProvider::Jira
        | IntegrationProvider::Monday
        | IntegrationProvider::ClickUp
        | IntegrationProvider::Trello => &[Capability::CreateTask],
        IntegrationProvider::HubSpot
        | IntegrationProvider::Attio
        | IntegrationProvider::Salesforce => &[Capability::LogActivity, Capability::CreateTask],
        IntegrationProvider::Affinity | IntegrationProvider::ZohoCrm => &[Capability::LogActivity],
        IntegrationProvider::Gmail | IntegrationProvider::OutlookMail => {
            &[Capability::ReadContext, Capability::PostMessage]
        }
        IntegrationProvider::Sandbox => &[],
    }
}

/// True for providers that are listed as coming soon when not shipped: all
/// but the sandbox, which only exists in debug builds.
pub const fn is_planned(provider: IntegrationProvider) -> bool {
    !matches!(provider, IntegrationProvider::Sandbox)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_planned_provider_says_what_it_will_do() {
        for &provider in IntegrationProvider::ALL {
            assert_eq!(
                is_planned(provider),
                !planned_capabilities(provider).is_empty(),
                "{provider}"
            );
        }
    }
}
