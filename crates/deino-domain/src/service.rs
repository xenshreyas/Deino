//! [`ServiceId`]: every outside service Deino may talk to (FND-12).
//!
//! Data leaves the Mac only for a service the user connected (PARITY_PLAN
//! §1). Each one is named here, so settings can switch it on or off, the
//! egress log can say where a request went, and `deino-net` can map it to the
//! hosts it may reach. The names are stored in `egress_log.service`, whose
//! `CHECK` list matches [`ServiceId::ALL`] exactly.

db_enum! {
    /// An outside service. Stored in `egress_log.service`; keys the
    /// `network.services` switches in the settings.
    ServiceId {
        /// Anthropic's Claude API.
        LlmAnthropic = "llm_anthropic",
        /// The OpenAI API.
        #[serde(rename = "llm_openai")]
        #[sqlx(rename = "llm_openai")]
        LlmOpenAi = "llm_openai",
        /// The Groq API.
        LlmGroq = "llm_groq",
        /// The OpenRouter API.
        #[serde(rename = "llm_openrouter")]
        #[sqlx(rename = "llm_openrouter")]
        LlmOpenRouter = "llm_openrouter",
        /// The user's own OpenAI-compatible endpoint; its host comes from the settings.
        LlmCustom = "llm_custom",
        /// An Ollama server: this Mac by default, or the endpoint in the settings.
        LlmOllama = "llm_ollama",
        /// Model files on Hugging Face (transcription and the built-in LLM).
        HfModels = "hf_models",
        /// Google Calendar (ORG-01).
        GoogleCalendar = "google_calendar",
        /// Google Drive and Docs (INT-05).
        GoogleDrive = "google_drive",
        /// Gmail (INT-07).
        Gmail = "gmail",
        /// Microsoft Graph: Outlook calendar and mail (ORG-01, INT-07).
        MicrosoftGraph = "microsoft_graph",
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
        /// HubSpot (INT-04).
        #[serde(rename = "hubspot")]
        #[sqlx(rename = "hubspot")]
        HubSpot = "hubspot",
        /// Attio (INT-04).
        Attio = "attio",
        /// Salesforce (INT-04).
        Salesforce = "salesforce",
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
        /// Affinity (INT-04).
        Affinity = "affinity",
        /// Zoho CRM (INT-04).
        ZohoCrm = "zoho_crm",
        /// Web search for chat with a user-provided key (D4).
        WebSearch = "web_search",
        /// Outbound webhooks to URLs the user enters (DEV-01).
        Webhooks = "webhooks",
        /// Company logos: `https://<domain>/favicon.ico`, fetched once per
        /// company (ORG-04).
        Favicons = "favicons",
        /// Granola's public API, for importing notes (CAP-12).
        Granola = "granola",
        /// Circleback's API, for importing meetings (CAP-12).
        Circleback = "circleback",
        /// Fireflies' GraphQL API, for importing transcripts (CAP-12).
        Fireflies = "fireflies",
        /// Fathom's API, for importing meetings (CAP-12).
        Fathom = "fathom",
        /// ElevenLabs speech to text: each meeting's audio, sent after it
        /// ends when the user turns it on with their own key (CAP-14).
        #[serde(rename = "elevenlabs")]
        #[sqlx(rename = "elevenlabs")]
        ElevenLabs = "elevenlabs",
    }
}

/// What kind of service a [`ServiceId`] is, for grouping in the UI.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, specta::Type,
)]
#[serde(rename_all = "snake_case")]
pub enum ServiceKind {
    /// An LLM provider for notes and chat.
    Llm,
    /// Model file downloads.
    Models,
    /// A connected app: calendar, mail, chat, docs, tasks or CRM.
    Integration,
    /// Web search, webhooks and company logos.
    Web,
}

impl ServiceId {
    /// The kind of service this is.
    pub const fn kind(self) -> ServiceKind {
        match self {
            Self::LlmAnthropic
            | Self::LlmOpenAi
            | Self::LlmGroq
            | Self::LlmOpenRouter
            | Self::LlmCustom
            | Self::LlmOllama => ServiceKind::Llm,
            Self::HfModels => ServiceKind::Models,
            Self::WebSearch | Self::Webhooks | Self::Favicons => ServiceKind::Web,
            _ => ServiceKind::Integration,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db_enum::assert_names_agree;

    #[test]
    fn serde_and_storage_names_agree() {
        assert_names_agree(ServiceId::ALL, ServiceId::as_str);
    }

    #[test]
    fn llm_providers_and_model_downloads_are_told_apart() {
        assert_eq!(ServiceId::LlmOllama.kind(), ServiceKind::Llm);
        assert_eq!(ServiceId::HfModels.kind(), ServiceKind::Models);
        assert_eq!(ServiceId::Slack.kind(), ServiceKind::Integration);
        assert_eq!(ServiceId::WebSearch.kind(), ServiceKind::Web);
    }
}
