//! The privacy and profile sections: placeholders that later cards fill.
//!
//! They exist now so the settings file has its final shape: PRV-01/PRV-02
//! add retention and app lock to [`PrivacySettings`], and NOT-05 adds the
//! user's profile and global AI context to [`ProfileSettings`]. Each new
//! field is `#[serde(default)]`, so existing files keep loading.

use serde::{Deserialize, Serialize};

/// The privacy section of `Settings`. Empty until PRV-01.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct PrivacySettings {}

/// The profile section of `Settings`. Empty until NOT-05.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct ProfileSettings {}
