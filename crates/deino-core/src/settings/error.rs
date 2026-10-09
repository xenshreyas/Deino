//! What can go wrong when reading, changing or saving settings.
//!
//! Messages name settings by their JSON path (`ai.ollamaEndpoint`), never
//! their values; the serde and I/O detail stays in the source chain.

use deino_domain::{ErrorClass, ErrorCode};

/// A settings failure.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SettingsError {
    /// A patch must be a JSON object.
    #[error("a settings change must be a JSON object")]
    NotAnObject,
    /// Patches may not change the format version.
    #[error("the settings version cannot be changed")]
    VersionIsReadOnly,
    /// The patch sets a path that only a dialog may choose.
    #[error("{0} is chosen with its dialog, not set directly")]
    ChosenInDialog(&'static str),
    /// The patch names a setting that does not exist.
    #[error("there is no setting named {0}")]
    UnknownSetting(String),
    /// The patched settings do not have the settings' shape (a wrong type, say).
    #[error("a setting has the wrong type")]
    WrongShape(#[source] serde_json::Error),
    /// A value breaks one of its setting's rules.
    #[error("{setting} {rule}")]
    InvalidValue {
        /// The setting's JSON path, for example `ai.ollamaEndpoint`.
        setting: &'static str,
        /// What is wrong, for example "must be an http:// or https:// URL".
        rule: &'static str,
    },
    /// The settings file could not be read.
    #[error("could not read the settings file")]
    Read(#[source] std::io::Error),
    /// The settings file could not be written.
    #[error("could not save the settings file")]
    Write(#[source] std::io::Error),
    /// The settings could not be encoded as JSON (a bug).
    #[error("could not encode the settings")]
    Encode(#[source] serde_json::Error),
}

impl SettingsError {
    /// A broken rule of `setting`.
    pub(crate) fn invalid(setting: &'static str, rule: &'static str) -> Self {
        Self::InvalidValue { setting, rule }
    }
}

impl ErrorClass for SettingsError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::NotAnObject
            | Self::VersionIsReadOnly
            | Self::ChosenInDialog(_)
            | Self::UnknownSetting(_)
            | Self::WrongShape(_)
            | Self::InvalidValue { .. } => ErrorCode::Invalid,
            Self::Read(_) | Self::Write(_) | Self::Encode(_) => ErrorCode::Internal,
        }
    }
}
