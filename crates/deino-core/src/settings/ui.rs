//! The UI and beta sections: appearance, layout and opt-in features.

use deino_domain::MeetingViewId;
use serde::{Deserialize, Serialize};

use super::SettingsError;
use crate::meetings::MeetingFilter;

/// The narrowest transcript pane on the meeting page, as a share of its width.
pub const MIN_TRANSCRIPT_PANE_RATIO: f32 = 0.3;
/// The widest transcript pane on the meeting page, as a share of its width.
pub const MAX_TRANSCRIPT_PANE_RATIO: f32 = 0.5;

/// The colour theme (PARITY_PLAN §3B: follow the system by default).
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type,
)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    /// Match macOS's appearance.
    #[default]
    System,
    /// Always light.
    Light,
    /// Always dark.
    Dark,
}

/// The UI section of `Settings`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct UiSettings {
    /// The colour theme.
    pub theme: Theme,
    /// Show the transcription confidence indicator next to transcript lines.
    pub show_confidence_indicator: bool,
    /// The transcript pane's share of the meeting page's width.
    pub transcript_pane_ratio: f32,
    /// The Meetings page's filter and open tab, kept across launches.
    pub meetings_list: MeetingsListPrefs,
}

/// The Meetings page as the user left it (ORG-07: "filters persist").
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct MeetingsListPrefs {
    /// The open saved view; `None` for "All meetings".
    pub view: Option<MeetingViewId>,
    /// The filter on screen, which may differ from the view's own.
    pub filter: MeetingFilter,
}

impl Default for UiSettings {
    fn default() -> Self {
        Self {
            theme: Theme::System,
            show_confidence_indicator: true,
            transcript_pane_ratio: MIN_TRANSCRIPT_PANE_RATIO,
            meetings_list: MeetingsListPrefs::default(),
        }
    }
}

impl UiSettings {
    /// Checks the pane ratio.
    pub(super) fn validate(&self) -> Result<(), SettingsError> {
        let ratios = MIN_TRANSCRIPT_PANE_RATIO..=MAX_TRANSCRIPT_PANE_RATIO;
        if !ratios.contains(&self.transcript_pane_ratio) {
            return Err(SettingsError::invalid(
                "ui.transcriptPaneRatio",
                "is outside its allowed range",
            ));
        }
        Ok(())
    }
}

/// The beta section of `Settings`: features still being proven.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct BetaSettings {
    /// Import audio files and re-transcribe meetings with another model.
    pub import_and_retranscribe: bool,
}

impl Default for BetaSettings {
    fn default() -> Self {
        Self {
            import_and_retranscribe: true,
        }
    }
}
