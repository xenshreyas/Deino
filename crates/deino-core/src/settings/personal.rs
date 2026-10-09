//! The privacy section: a placeholder that PRV-01 fills.
//!
//! It exists now so the settings file has its final shape: PRV-01/PRV-02 add
//! retention and app lock to [`PrivacySettings`]. Each new field is
//! `#[serde(default)]`, so existing files keep loading.

use serde::{Deserialize, Serialize};

/// The privacy section of `Settings`. Empty until PRV-01.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct PrivacySettings {}
