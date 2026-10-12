//! Task routes: every LLM call says what it is for (PARITY_PLAN §3.6, §4.3).
//!
//! A route maps to a provider and model through the user's route settings,
//! with the defaults of §4.3 ([`crate::Router`]). Adding a route is one
//! variant here, its name, its tier, and a migration that widens the
//! `llm_calls.route` CHECK constraint.

use std::fmt;

use serde::{Deserialize, Serialize};

/// What an LLM call is for. Serialized as the dotted names of §4.3.
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    specta::Type,
    sqlx::Type,
)]
pub enum Route {
    /// Enhanced notes from a transcript and the user's notes (NOT-02).
    #[serde(rename = "notes.enhance")]
    #[sqlx(rename = "notes.enhance")]
    NotesEnhance,
    /// Editing notes by asking (NOT-06).
    #[serde(rename = "notes.edit")]
    #[sqlx(rename = "notes.edit")]
    NotesEdit,
    /// Chat across meetings (AI-02).
    #[serde(rename = "chat")]
    #[sqlx(rename = "chat")]
    Chat,
    /// Action item extraction (AI-05).
    #[serde(rename = "extract.action_items")]
    #[sqlx(rename = "extract.action_items")]
    ExtractActionItems,
    /// Custom insights (AI-06).
    #[serde(rename = "extract.insights")]
    #[sqlx(rename = "extract.insights")]
    ExtractInsights,
    /// AI auto-tagging (ORG-06).
    #[serde(rename = "tagging")]
    #[sqlx(rename = "tagging")]
    Tagging,
    /// Meeting titles (NOT-02).
    #[serde(rename = "title")]
    #[sqlx(rename = "title")]
    Title,
    /// People and company summaries (ORG-04).
    #[serde(rename = "entity.summary")]
    #[sqlx(rename = "entity.summary")]
    EntitySummary,
    /// Pre-meeting briefs (NOT-08).
    #[serde(rename = "brief")]
    #[sqlx(rename = "brief")]
    Brief,
    /// Follow-up email drafts (NOT-07).
    #[serde(rename = "followup.email")]
    #[sqlx(rename = "followup.email")]
    FollowupEmail,
    /// The year in review, from aggregated stats and meeting titles (ORG-09).
    #[serde(rename = "recap")]
    #[sqlx(rename = "recap")]
    Recap,
    /// A shared-screen snapshot described in words (CAP-09); a picture
    /// goes with the prompt, so only Claude serves it.
    #[serde(rename = "vision.describe")]
    #[sqlx(rename = "vision.describe")]
    VisionDescribe,
    /// Embeddings for semantic search (AI-01); always local (§8 D3).
    #[serde(rename = "embed")]
    #[sqlx(rename = "embed")]
    Embed,
}

/// How much model a route needs, which picks its default model.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum RouteTier {
    /// Long-form writing and reasoning: notes, chat, briefs, emails.
    Smart,
    /// Short extraction and classification: titles, tags, action items.
    Fast,
    /// Vectors, never text: always the local embedding model.
    Embedding,
}

impl Route {
    /// Every route, in the order Settings → AI lists them.
    pub const ALL: [Self; 13] = [
        Self::NotesEnhance,
        Self::NotesEdit,
        Self::Chat,
        Self::ExtractActionItems,
        Self::ExtractInsights,
        Self::Tagging,
        Self::Title,
        Self::EntitySummary,
        Self::Brief,
        Self::FollowupEmail,
        Self::Recap,
        Self::VisionDescribe,
        Self::Embed,
    ];

    /// The route's dotted name, as stored and as §4.3 spells it.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NotesEnhance => "notes.enhance",
            Self::NotesEdit => "notes.edit",
            Self::Chat => "chat",
            Self::ExtractActionItems => "extract.action_items",
            Self::ExtractInsights => "extract.insights",
            Self::Tagging => "tagging",
            Self::Title => "title",
            Self::EntitySummary => "entity.summary",
            Self::Brief => "brief",
            Self::FollowupEmail => "followup.email",
            Self::Recap => "recap",
            Self::VisionDescribe => "vision.describe",
            Self::Embed => "embed",
        }
    }

    /// The tier that picks the route's default model (§4.3).
    pub fn tier(self) -> RouteTier {
        match self {
            Self::NotesEnhance
            | Self::NotesEdit
            | Self::Chat
            | Self::Brief
            | Self::FollowupEmail
            | Self::Recap => RouteTier::Smart,
            Self::ExtractActionItems
            | Self::ExtractInsights
            | Self::Tagging
            | Self::Title
            | Self::EntitySummary
            | Self::VisionDescribe => RouteTier::Fast,
            Self::Embed => RouteTier::Embedding,
        }
    }
}

impl fmt::Display for Route {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routes_serialize_as_their_dotted_names() {
        for route in Route::ALL {
            let json = serde_json::to_value(route).unwrap();
            assert_eq!(json, route.as_str(), "{route:?}");
            assert_eq!(serde_json::from_value::<Route>(json).unwrap(), route);
        }
    }

    #[test]
    fn tiers_follow_the_default_routing_of_section_4_3() {
        let smart: Vec<&str> = Route::ALL
            .into_iter()
            .filter(|route| route.tier() == RouteTier::Smart)
            .map(Route::as_str)
            .collect();
        let fast: Vec<&str> = Route::ALL
            .into_iter()
            .filter(|route| route.tier() == RouteTier::Fast)
            .map(Route::as_str)
            .collect();

        assert_eq!(
            smart,
            [
                "notes.enhance",
                "notes.edit",
                "chat",
                "brief",
                "followup.email",
                "recap"
            ]
        );
        assert_eq!(
            fast,
            [
                "extract.action_items",
                "extract.insights",
                "tagging",
                "title",
                "entity.summary",
                "vision.describe"
            ]
        );
        assert_eq!(Route::Embed.tier(), RouteTier::Embedding);
    }
}
