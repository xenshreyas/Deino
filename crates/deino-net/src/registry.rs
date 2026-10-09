//! The service registry: the hosts each [`ServiceId`] may reach, and its
//! defaults.
//!
//! A request to a host that is not listed for its service fails before
//! anything is sent. A service that takes its endpoint from the settings
//! (`custom_hosts`) adds the configured host through the `ServicePolicy`.
//! Adding a service is one `ServiceId` variant plus one arm here.

use deino_domain::ServiceId;

use crate::HostPattern;

/// What Deino knows about a service before the user changes anything.
#[derive(Clone, Copy, Debug)]
pub struct ServiceSpec {
    /// The hosts the service may always reach.
    pub hosts: &'static [HostPattern],
    /// Whether the service may send before the user switches it on: true
    /// only for services that send nothing until the user sets them up (an
    /// LLM provider needs a key or an endpoint; a model download needs a click).
    pub enabled_by_default: bool,
    /// Whether the user's settings add a host (a custom endpoint).
    pub custom_hosts: bool,
    /// Whether downloads the user starts may run in offline mode.
    pub offline_downloads: bool,
}

/// The registry entry of `service`.
pub const fn spec(service: ServiceId) -> ServiceSpec {
    match service {
        ServiceId::LlmAnthropic => llm(hosts::ANTHROPIC),
        ServiceId::LlmOpenAi => llm(hosts::OPENAI),
        ServiceId::LlmGroq => llm(hosts::GROQ),
        ServiceId::LlmOpenRouter => llm(hosts::OPENROUTER),
        ServiceId::LlmCustom => ServiceSpec {
            custom_hosts: true,
            ..llm(&[])
        },
        ServiceId::LlmOllama => ServiceSpec {
            custom_hosts: true,
            ..llm(hosts::LOOPBACK)
        },
        ServiceId::HfModels => ServiceSpec {
            offline_downloads: true,
            ..llm(hosts::HUGGING_FACE)
        },
        _ => integration(service),
    }
}

/// Integrations, web search and webhooks: off until the user connects them.
const fn integration(service: ServiceId) -> ServiceSpec {
    let hosts = match service {
        ServiceId::GoogleCalendar => hosts::GOOGLE_CALENDAR,
        ServiceId::GoogleDrive => hosts::GOOGLE_DRIVE,
        ServiceId::Gmail => hosts::GMAIL,
        ServiceId::MicrosoftGraph => hosts::MICROSOFT_GRAPH,
        ServiceId::Slack => hosts::SLACK,
        ServiceId::Notion => hosts::NOTION,
        ServiceId::Linear => hosts::LINEAR,
        ServiceId::Todoist => hosts::TODOIST,
        ServiceId::Asana => hosts::ASANA,
        ServiceId::HubSpot => hosts::HUBSPOT,
        ServiceId::Attio => hosts::ATTIO,
        ServiceId::Salesforce => hosts::SALESFORCE,
        ServiceId::WebSearch => hosts::WEB_SEARCH,
        _ => &[],
    };
    ServiceSpec {
        hosts,
        enabled_by_default: false,
        custom_hosts: matches!(service, ServiceId::Webhooks),
        offline_downloads: false,
    }
}

/// An LLM provider (or model host): on by default, nothing custom.
const fn llm(hosts: &'static [HostPattern]) -> ServiceSpec {
    ServiceSpec {
        hosts,
        enabled_by_default: true,
        custom_hosts: false,
        offline_downloads: false,
    }
}

/// Each service's hosts.
mod hosts {
    use std::borrow::Cow;

    use crate::HostPattern;

    pub(super) const ANTHROPIC: &[HostPattern] = &[exact("api.anthropic.com")];
    pub(super) const OPENAI: &[HostPattern] = &[exact("api.openai.com")];
    pub(super) const GROQ: &[HostPattern] = &[exact("api.groq.com")];
    pub(super) const OPENROUTER: &[HostPattern] = &[exact("openrouter.ai")];
    /// This Mac, by name and by address.
    pub(super) const LOOPBACK: &[HostPattern] =
        &[exact("localhost"), exact("127.0.0.1"), exact("[::1]")];
    /// Hugging Face and the CDNs its `resolve/` URLs redirect to.
    pub(super) const HUGGING_FACE: &[HostPattern] = &[
        exact("huggingface.co"),
        HostPattern::SubdomainsOf("huggingface.co"),
        HostPattern::SubdomainsOf("hf.co"),
    ];
    pub(super) const GOOGLE_CALENDAR: &[HostPattern] =
        &[exact("www.googleapis.com"), GOOGLE_OAUTH];
    pub(super) const GOOGLE_DRIVE: &[HostPattern] = &[
        exact("www.googleapis.com"),
        exact("docs.googleapis.com"),
        GOOGLE_OAUTH,
    ];
    pub(super) const GMAIL: &[HostPattern] = &[exact("gmail.googleapis.com"), GOOGLE_OAUTH];
    pub(super) const MICROSOFT_GRAPH: &[HostPattern] = &[
        exact("graph.microsoft.com"),
        exact("login.microsoftonline.com"),
    ];
    pub(super) const SLACK: &[HostPattern] =
        &[exact("slack.com"), HostPattern::SubdomainsOf("slack.com")];
    pub(super) const NOTION: &[HostPattern] = &[exact("api.notion.com")];
    pub(super) const LINEAR: &[HostPattern] = &[exact("api.linear.app")];
    pub(super) const TODOIST: &[HostPattern] = &[exact("api.todoist.com")];
    pub(super) const ASANA: &[HostPattern] = &[exact("app.asana.com")];
    pub(super) const HUBSPOT: &[HostPattern] = &[exact("api.hubapi.com")];
    pub(super) const ATTIO: &[HostPattern] = &[exact("api.attio.com")];
    pub(super) const SALESFORCE: &[HostPattern] = &[
        exact("login.salesforce.com"),
        HostPattern::SubdomainsOf("salesforce.com"),
    ];
    pub(super) const WEB_SEARCH: &[HostPattern] =
        &[exact("api.search.brave.com"), exact("api.tavily.com")];

    /// Google's OAuth token endpoint, shared by every Google service.
    const GOOGLE_OAUTH: HostPattern = exact("oauth2.googleapis.com");

    /// A static exact pattern.
    const fn exact(host: &'static str) -> HostPattern {
        HostPattern::Exact(Cow::Borrowed(host))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_service_can_reach_somewhere() {
        for &service in ServiceId::ALL {
            let spec = spec(service);
            assert!(
                !spec.hosts.is_empty() || spec.custom_hosts,
                "{service} has no hosts"
            );
        }
    }

    #[test]
    fn only_llm_providers_and_model_downloads_send_by_default() {
        let on: Vec<ServiceId> = ServiceId::ALL
            .iter()
            .copied()
            .filter(|&service| spec(service).enabled_by_default)
            .collect();

        assert!(on.contains(&ServiceId::LlmAnthropic));
        assert!(on.contains(&ServiceId::HfModels));
        assert!(!on.contains(&ServiceId::Slack));
        assert!(!on.contains(&ServiceId::WebSearch));
    }

    #[test]
    fn only_model_downloads_may_run_offline() {
        let offline: Vec<ServiceId> = ServiceId::ALL
            .iter()
            .copied()
            .filter(|&service| spec(service).offline_downloads)
            .collect();
        assert_eq!(offline, [ServiceId::HfModels]);
    }

    #[test]
    fn hugging_face_covers_its_cdn_redirects() {
        let hosts = spec(ServiceId::HfModels).hosts;
        for host in ["huggingface.co", "cdn-lfs.huggingface.co", "cas-bridge.xethub.hf.co"] {
            assert!(hosts.iter().any(|pattern| pattern.matches(host)), "{host}");
        }
    }
}
