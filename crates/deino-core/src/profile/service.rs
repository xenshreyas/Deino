//! [`ProfileService`]: saves the profile and keeps "Me" in step with it.

use std::sync::Arc;

use deino_domain::{Clock, PersonId};

use super::me::MeSync;
use super::{ProfileError, ProfileInput, render_profile_context};
use crate::people::{MeRepo, PersonRepo};
use crate::settings::{ProfileSettings, SettingsService};

/// Reads and changes the user's profile.
pub struct ProfileService {
    settings: Arc<SettingsService>,
    me: MeSync,
}

impl ProfileService {
    /// Creates the service over the settings and the people ports.
    pub fn new(
        settings: Arc<SettingsService>,
        people: Arc<dyn PersonRepo>,
        me: Arc<dyn MeRepo>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            settings,
            me: MeSync { people, me, clock },
        }
    }

    /// The saved profile.
    pub async fn get(&self) -> ProfileSettings {
        self.settings.get().await.profile
    }

    /// Normalises and saves `input` as the profile, then updates "Me".
    ///
    /// The settings are saved first: a profile that breaks a rule changes
    /// nothing, and a failed person write leaves a saved profile that the
    /// next start ([`ProfileService::sync_me`]) reconciles.
    pub async fn update(&self, input: ProfileInput) -> Result<ProfileSettings, ProfileError> {
        let profile = input.normalize()?;
        let saved = profile.clone();
        self.settings
            .update(move |settings| settings.profile = saved)
            .await?;
        self.me.sync(&profile).await?;
        Ok(profile)
    }

    /// Makes sure the "Me" person exists and matches the saved profile.
    /// Runs at startup, so a fresh install has "Me" before any meeting.
    pub async fn sync_me(&self) -> Result<PersonId, ProfileError> {
        let profile = self.get().await;
        Ok(self.me.sync(&profile).await?)
    }

    /// The system-context snippet for the saved profile.
    pub async fn context(&self) -> Option<String> {
        render_profile_context(&self.get().await)
    }
}
