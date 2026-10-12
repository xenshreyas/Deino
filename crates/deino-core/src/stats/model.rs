//! What the Stats page shows: the numbers of a period, the year's heatmap,
//! LLM usage and the year in review. Plain data, serialized for the UI.

use chrono::{DateTime, NaiveDate, Utc};
use deino_domain::PersonId;
use deino_llm::ProviderId;
use serde::{Deserialize, Serialize};

/// The period the Stats page covers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum StatsRange {
    /// Today and the 29 days before it.
    #[serde(rename = "last_30_days")]
    Last30Days,
    /// Today and the 89 days before it.
    #[serde(rename = "last_90_days")]
    Last90Days,
    /// From this day a year ago.
    #[serde(rename = "last_12_months")]
    Last12Months,
    /// Everything.
    AllTime,
}

/// The numbers of one period.
#[derive(Clone, Debug, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct StatsOverview {
    /// Where the period starts; `None` for all time.
    pub since: Option<DateTime<Utc>>,
    /// Meetings in the period.
    pub meetings: u32,
    /// Their recorded length, in seconds.
    pub recorded_seconds: u64,
    /// Different people met, not counting the user.
    pub people: u32,
    /// The meetings by who they were with.
    pub audience: AudienceSplit,
    /// The people met most, most meetings first.
    pub top_people: Vec<TopPerson>,
    /// Meetings per weekday, Monday first (7 entries).
    pub weekdays: Vec<u32>,
    /// Meetings per starting hour, local time, midnight first (24 entries).
    pub hours: Vec<u32>,
    /// LLM calls in the period.
    pub llm: LlmUsage,
}

/// Meetings by who they were with (`profile::InternalDomains`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AudienceSplit {
    /// Only the user's organisation.
    pub internal: u32,
    /// At least one person from outside it.
    pub external: u32,
    /// Not known: no attendee list, or no address in the user's profile.
    pub unknown: u32,
}

/// Someone the user met often.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TopPerson {
    /// The person.
    pub id: PersonId,
    /// Their name.
    pub name: String,
    /// Meetings with them in the period.
    pub meetings: u32,
}

/// LLM usage, from the gateway's `llm_calls` log.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct LlmUsage {
    /// One row per provider and model, most calls first.
    pub models: Vec<ModelUsage>,
    /// The estimated cost of every priced row, in millionths of a US dollar.
    pub estimated_cost_micros: u64,
    /// Calls to models with no known price, left out of the estimate.
    pub unpriced_calls: u32,
}

/// The calls to one model.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ModelUsage {
    /// The provider.
    pub provider: ProviderId,
    /// The provider's model ID.
    pub model: String,
    /// Calls made.
    pub calls: u32,
    /// Calls that failed.
    pub failed: u32,
    /// Prompt tokens.
    pub input_tokens: u64,
    /// Answer tokens.
    pub output_tokens: u64,
    /// The estimated cost in millionths of a US dollar, from list prices;
    /// `None` when the model's price is not known. Local models cost 0.
    pub estimated_cost_micros: Option<u64>,
}

/// Meetings per day over the last year, for the GitHub-style heatmap.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Heatmap {
    /// The first day shown: a Monday, so the grid's columns are weeks.
    pub first_day: NaiveDate,
    /// Today, the last day shown.
    pub last_day: NaiveDate,
    /// The days with at least one meeting, oldest first.
    pub days: Vec<HeatmapDay>,
}

/// One day of the [`Heatmap`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct HeatmapDay {
    /// The local day.
    pub day: NaiveDate,
    /// Meetings that started on it.
    pub meetings: u32,
}

/// A generated year in review.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct YearRecap {
    /// The calendar year it covers.
    pub year: i32,
    /// The recap, as Markdown.
    pub markdown: String,
    /// When it was generated.
    pub generated_at: DateTime<Utc>,
}

/// Whether this year's recap can be read or generated.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RecapStatus {
    /// The current calendar year.
    pub year: i32,
    /// True in December, or any time with the debug flag.
    pub available: bool,
    /// The recap, once generated.
    pub recap: Option<YearRecap>,
}
