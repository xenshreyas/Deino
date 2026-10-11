//! The closed sets of the integrations framework (INT-00, ADR 0029).
//!
//! Each enum's names match the `CHECK` list of the column named in its docs
//! (`integration_connections`, `integration_deliveries`). A provider card
//! (INT-01..07) implements a provider that is already listed here, so it
//! needs no migration to store its connections.

use crate::ServiceId;

db_enum! {
    /// A service Deino can connect to. Stored in
    /// `integration_connections.provider` and `integration_deliveries.provider`.
    IntegrationProvider {
        /// Slack (INT-01).
        Slack = "slack",
        /// Notion (INT-02).
        Notion = "notion",
        /// Linear (INT-03).
        Linear = "linear",
        /// Todoist (INT-03).
        Todoist = "todoist",
        /// Asana (INT-03).
        Asana = "asana",
        /// Jira Cloud (INT-03).
        Jira = "jira",
        /// monday.com (INT-03).
        Monday = "monday",
        /// ClickUp (INT-03).
        #[serde(rename = "clickup")]
        #[sqlx(rename = "clickup")]
        ClickUp = "clickup",
        /// Trello (INT-03).
        Trello = "trello",
        /// HubSpot (INT-04).
        #[serde(rename = "hubspot")]
        #[sqlx(rename = "hubspot")]
        HubSpot = "hubspot",
        /// Attio (INT-04).
        Attio = "attio",
        /// Salesforce (INT-04).
        Salesforce = "salesforce",
        /// Affinity (INT-04).
        Affinity = "affinity",
        /// Zoho CRM (INT-04).
        ZohoCrm = "zoho_crm",
        /// Google Docs in Drive (INT-05).
        GoogleDocs = "google_docs",
        /// Gmail (INT-07).
        Gmail = "gmail",
        /// Outlook mail through Microsoft Graph (INT-07).
        OutlookMail = "outlook_mail",
        /// A provider that talks to nothing: debug builds and tests only.
        Sandbox = "sandbox",
    }
}

impl IntegrationProvider {
    /// The provider's name as people write it, for messages and the UI.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Slack => "Slack",
            Self::Notion => "Notion",
            Self::Linear => "Linear",
            Self::Todoist => "Todoist",
            Self::Asana => "Asana",
            Self::Jira => "Jira",
            Self::Monday => "monday.com",
            Self::ClickUp => "ClickUp",
            Self::Trello => "Trello",
            Self::HubSpot => "HubSpot",
            Self::Attio => "Attio",
            Self::Salesforce => "Salesforce",
            Self::Affinity => "Affinity",
            Self::ZohoCrm => "Zoho CRM",
            Self::GoogleDocs => "Google Docs",
            Self::Gmail => "Gmail",
            Self::OutlookMail => "Outlook",
            Self::Sandbox => "Sandbox",
        }
    }

    /// The network service the provider's requests go through; `None` for
    /// the sandbox, which never sends anything.
    pub const fn service(self) -> Option<ServiceId> {
        Some(match self {
            Self::Slack => ServiceId::Slack,
            Self::Notion => ServiceId::Notion,
            Self::Linear => ServiceId::Linear,
            Self::Todoist => ServiceId::Todoist,
            Self::Asana => ServiceId::Asana,
            Self::Jira => ServiceId::Jira,
            Self::Monday => ServiceId::Monday,
            Self::ClickUp => ServiceId::ClickUp,
            Self::Trello => ServiceId::Trello,
            Self::HubSpot => ServiceId::HubSpot,
            Self::Attio => ServiceId::Attio,
            Self::Salesforce => ServiceId::Salesforce,
            Self::Affinity => ServiceId::Affinity,
            Self::ZohoCrm => ServiceId::ZohoCrm,
            Self::GoogleDocs => ServiceId::GoogleDrive,
            Self::Gmail => ServiceId::Gmail,
            Self::OutlookMail => ServiceId::MicrosoftGraph,
            Self::Sandbox => return None,
        })
    }
}

db_enum! {
    /// How a connection was made. Stored in `integration_connections.auth_method`.
    AuthMethod {
        /// OAuth 2.0 with PKCE through a loopback redirect.
        #[serde(rename = "oauth")]
        #[sqlx(rename = "oauth")]
        OAuth = "oauth",
        /// A token or API key the user pasted.
        Token = "token",
    }
}

db_enum! {
    /// Whether a connection can be used. Stored in `integration_connections.status`.
    ConnectionStatus {
        /// The credential works, as far as Deino knows.
        Connected = "connected",
        /// The provider rejected the credential and it could not be
        /// refreshed: the user has to connect again.
        NeedsReconnect = "needs_reconnect",
    }
}

db_enum! {
    /// Something a provider can do for Deino. Write capabilities are stored in
    /// `integration_deliveries.capability`.
    Capability {
        /// Post a message, with replies in a thread (Slack).
        PostMessage = "post_message",
        /// Create a page or document (Notion, Google Docs).
        CreatePage = "create_page",
        /// Create a task (Linear, Todoist, Asana, ...).
        CreateTask = "create_task",
        /// Log a meeting or note on CRM records (HubSpot, Attio, ...).
        LogActivity = "log_activity",
        /// Read context on demand, never stored (Gmail threads, Linear issues).
        ReadContext = "read_context",
    }
}

impl Capability {
    /// True for capabilities that write to the provider, which are
    /// delivered once per idempotency key and logged.
    pub const fn writes(self) -> bool {
        !matches!(self, Self::ReadContext)
    }
}

db_enum! {
    /// Where a delivery stands. Stored in `integration_deliveries.status`.
    DeliveryStatus {
        /// Recorded, not sent yet.
        Pending = "pending",
        /// Being sent right now.
        Sending = "sending",
        /// The provider accepted it.
        Succeeded = "succeeded",
        /// The last attempt failed; sending the same key again retries it.
        Failed = "failed",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db_enum::assert_names_agree;

    #[test]
    fn serde_and_storage_names_agree() {
        assert_names_agree(IntegrationProvider::ALL, IntegrationProvider::as_str);
        assert_names_agree(AuthMethod::ALL, AuthMethod::as_str);
        assert_names_agree(ConnectionStatus::ALL, ConnectionStatus::as_str);
        assert_names_agree(Capability::ALL, Capability::as_str);
        assert_names_agree(DeliveryStatus::ALL, DeliveryStatus::as_str);
    }

    #[test]
    fn every_provider_but_the_sandbox_sends_through_an_integration_service() {
        for &provider in IntegrationProvider::ALL {
            match provider.service() {
                Some(service) => assert_eq!(service.kind(), crate::ServiceKind::Integration),
                None => assert_eq!(provider, IntegrationProvider::Sandbox),
            }
        }
    }

    #[test]
    fn only_reading_context_is_not_a_delivery() {
        let reads: Vec<_> = Capability::ALL.iter().filter(|c| !c.writes()).collect();
        assert_eq!(reads, [&Capability::ReadContext]);
    }
}
