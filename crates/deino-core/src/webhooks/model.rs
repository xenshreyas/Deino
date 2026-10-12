//! The webhook types: [`WebhookEvent`], the validated [`WebhookUrl`], an
//! endpoint ([`Webhook`]) and one message's log entry ([`Delivery`]).

use chrono::{DateTime, Utc};
use deino_domain::WebhookId;
use serde::{Deserialize, Serialize};
use url::Url;

use super::WebhookError;
use crate::automations::PayloadFields;

/// The longest endpoint URL, in bytes.
pub const MAX_URL_BYTES: usize = 2048;
/// The longest endpoint label, in characters.
pub const MAX_LABEL_CHARS: usize = 80;

/// Something that happened in Deino that an endpoint can subscribe to.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, specta::Type,
)]
pub enum WebhookEvent {
    /// The post-meeting pipeline finished: the meeting is ready.
    #[serde(rename = "meeting.ready")]
    MeetingReady,
    /// AI notes were written for the first time.
    #[serde(rename = "notes.generated")]
    NotesGenerated,
    /// AI notes were edited or written again.
    #[serde(rename = "notes.edited")]
    NotesEdited,
    /// An action item was added, by the user or by extraction.
    #[serde(rename = "action_item.created")]
    ActionItemCreated,
    /// An action item changed, was completed or reopened.
    #[serde(rename = "action_item.updated")]
    ActionItemUpdated,
    /// Custom insights were answered on a meeting.
    #[serde(rename = "insight.generated")]
    InsightGenerated,
}

impl WebhookEvent {
    /// Every event, in the order the page lists them.
    pub const ALL: [Self; 6] = [
        Self::MeetingReady,
        Self::NotesGenerated,
        Self::NotesEdited,
        Self::ActionItemCreated,
        Self::ActionItemUpdated,
        Self::InsightGenerated,
    ];

    /// The event's name on the wire and in `webhook_deliveries.event`.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MeetingReady => "meeting.ready",
            Self::NotesGenerated => "notes.generated",
            Self::NotesEdited => "notes.edited",
            Self::ActionItemCreated => "action_item.created",
            Self::ActionItemUpdated => "action_item.updated",
            Self::InsightGenerated => "insight.generated",
        }
    }

    /// True for events that fire in bursts (autosaves, ticking boxes): a
    /// new one joins a delivery still waiting for its first attempt.
    pub const fn coalesces(self) -> bool {
        matches!(self, Self::NotesEdited | Self::ActionItemUpdated)
    }
}

/// What a delivery carries: a subscribed event, an automation step's
/// payload, or a test.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(untagged)]
pub enum DeliveryKind {
    /// A subscribed event.
    Event(WebhookEvent),
    /// Something else.
    Other(OtherKind),
}

/// The deliveries that are not subscribed events.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum OtherKind {
    /// An automation's webhook step (AUT-01).
    Automation,
    /// "Send test" on the Webhooks page.
    Test,
}

impl DeliveryKind {
    /// The stored name.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Event(event) => event.as_str(),
            Self::Other(OtherKind::Automation) => "automation",
            Self::Other(OtherKind::Test) => "test",
        }
    }

    /// The kind stored as `name`.
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "automation" => Some(Self::Other(OtherKind::Automation)),
            "test" => Some(Self::Other(OtherKind::Test)),
            _ => WebhookEvent::ALL
                .into_iter()
                .find(|event| event.as_str() == name)
                .map(Self::Event),
        }
    }
}

/// An endpoint URL: `https`, or `http` on a loopback host for testing; no
/// user name or password in it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WebhookUrl(Url);

impl WebhookUrl {
    /// Validates `input` (trimmed).
    pub fn parse(input: &str) -> Result<Self, WebhookError> {
        let input = input.trim();
        let invalid = |rule| WebhookError::invalid("URL", rule);
        if input.len() > MAX_URL_BYTES {
            return Err(invalid("is too long"));
        }
        let url = Url::parse(input).map_err(|_| invalid("is not a valid address"))?;
        let host = url.host_str().unwrap_or_default();
        let loopback = matches!(host, "localhost" | "127.0.0.1" | "[::1]");
        match url.scheme() {
            "https" if !host.is_empty() => {}
            "http" if loopback => {}
            _ => {
                return Err(invalid(
                    "must start with https:// (http:// only for localhost)",
                ));
            }
        }
        if !url.username().is_empty() || url.password().is_some() {
            return Err(invalid("cannot contain a user name or password"));
        }
        Ok(Self(url))
    }

    /// The URL as text.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// The host the network policy allows while the endpoint exists.
    pub fn host(&self) -> &str {
        self.0.host_str().unwrap_or_default()
    }
}

/// Why Deino turned an endpoint off by itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum DisabledReason {
    /// Every attempt failed for a day.
    Failing,
    /// The endpoint answered 410 Gone.
    Gone,
}

impl DisabledReason {
    /// The stored name.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Failing => "failing",
            Self::Gone => "gone",
        }
    }

    /// The reason stored as `name`.
    pub fn parse(name: &str) -> Option<Self> {
        [Self::Failing, Self::Gone]
            .into_iter()
            .find(|reason| reason.as_str() == name)
    }
}

/// An endpoint, as stored and as the page shows it (never its secret).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Webhook {
    /// Its ID.
    pub id: WebhookId,
    /// Where it posts.
    pub url: String,
    /// What the user calls it; may be empty.
    pub label: String,
    /// The events it receives.
    pub events: Vec<WebhookEvent>,
    /// What its payload carries.
    pub fields: PayloadFields,
    /// Off: receives nothing.
    pub enabled: bool,
    /// Set when Deino turned it off after it kept failing.
    pub disabled_reason: Option<DisabledReason>,
    /// Attempts failed in a row since the last success.
    pub failure_streak: u32,
    /// When the current failure streak began.
    pub failing_since: Option<DateTime<Utc>>,
    /// The last successful delivery.
    pub last_success_at: Option<DateTime<Utc>>,
    /// When it was added.
    pub created_at: DateTime<Utc>,
    /// When it last changed.
    pub updated_at: DateTime<Utc>,
}

/// An endpoint as the user writes it on the page.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct WebhookInput {
    /// Where to post.
    pub url: String,
    /// What to call it.
    pub label: String,
    /// The events it receives (may be empty: automations and tests only).
    pub events: Vec<WebhookEvent>,
    /// What its payload carries.
    pub fields: PayloadFields,
    /// On or off.
    pub enabled: bool,
}

/// A newly added endpoint and its signing secret, shown this once.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CreatedWebhook {
    /// The endpoint.
    pub webhook: Webhook,
    /// `whsec_…`: the page shows it once; it lives only in the Keychain.
    pub secret: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urls_must_be_https_except_on_this_mac() {
        for good in [
            "https://hooks.zapier.com/hooks/catch/1/abc/",
            " https://hook.eu1.make.com/xyz ",
            "http://localhost:8080/hook",
            "http://127.0.0.1:9/x",
        ] {
            assert!(WebhookUrl::parse(good).is_ok(), "{good}");
        }
        for bad in [
            "http://example.com/hook",
            "ftp://example.com",
            "hooks.zapier.com",
            "https://user:pass@example.com/",
            "",
        ] {
            assert!(WebhookUrl::parse(bad).is_err(), "{bad}");
        }
        let url = WebhookUrl::parse("https://Hooks.Zapier.com/a").unwrap();
        assert_eq!(url.host(), "hooks.zapier.com");
    }

    #[test]
    fn kinds_round_trip_through_their_stored_names() {
        let kinds = WebhookEvent::ALL
            .map(DeliveryKind::Event)
            .into_iter()
            .chain([OtherKind::Automation, OtherKind::Test].map(DeliveryKind::Other));
        for kind in kinds {
            assert_eq!(DeliveryKind::parse(kind.as_str()), Some(kind));
        }
        assert_eq!(DeliveryKind::parse("nope"), None);
        let json = serde_json::to_string(&WebhookEvent::ActionItemCreated).unwrap();
        assert_eq!(json, r#""action_item.created""#);
    }
}
