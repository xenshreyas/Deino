//! [`HotkeyService`]: registers the saved hotkeys and changes them.

use std::collections::BTreeMap;
use std::sync::Arc;

use super::accelerator::Accelerator;
use super::error::HotkeyError;
use super::model::{Hotkey, HotkeyAction, HotkeyState};
use super::ports::HotkeyRegistrar;
use crate::settings::{SettingsService, ShortcutSettings};

/// Each action's state.
type States = BTreeMap<HotkeyAction, HotkeyState>;

/// Keeps the system's hotkeys in step with `settings.shortcuts`.
///
/// A hotkey is registered before it is saved, so a combination another app
/// holds is refused instead of saved and silently dead. A saved hotkey
/// that fails at startup (now held by another app, or edited by hand into
/// something invalid) keeps its value and reports why it does not work.
pub struct HotkeyService {
    registrar: Arc<dyn HotkeyRegistrar>,
    settings: Arc<SettingsService>,
    states: tokio::sync::Mutex<States>,
}

impl HotkeyService {
    /// A service over the system's hotkeys and the settings.
    pub fn new(registrar: Arc<dyn HotkeyRegistrar>, settings: Arc<SettingsService>) -> Self {
        Self {
            registrar,
            settings,
            states: tokio::sync::Mutex::new(States::new()),
        }
    }

    /// Registers every saved hotkey; run once at startup.
    pub async fn start(&self) -> Vec<Hotkey> {
        let saved = self.settings.get().await.shortcuts;
        let mut states = self.states.lock().await;
        let mut claimed: Vec<(Accelerator, HotkeyAction)> = Vec::new();
        for action in HotkeyAction::ALL {
            let state = match action.saved(&saved) {
                None => HotkeyState::Off,
                Some(text) => self
                    .claim(action, text, &mut claimed)
                    .map_or_else(|error| failed(&error), |()| HotkeyState::Active),
            };
            states.insert(action, state);
        }
        listing(&saved, &states)
    }

    /// Every hotkey, saved value and state.
    pub async fn list(&self) -> Vec<Hotkey> {
        let saved = self.settings.get().await.shortcuts;
        listing(&saved, &*self.states.lock().await)
    }

    /// Gives `action` the hotkey `accelerator`, or turns it off with
    /// `None`: validated, checked against the other hotkeys, registered,
    /// then saved. On any failure the previous hotkey keeps working.
    pub async fn set(
        &self,
        action: HotkeyAction,
        accelerator: Option<&str>,
    ) -> Result<Vec<Hotkey>, HotkeyError> {
        let mut states = self.states.lock().await;
        let saved = self.settings.get().await.shortcuts;
        let next = accelerator.map(str::parse::<Accelerator>).transpose()?;
        if let Some(next) = &next {
            check_free(action, next, &saved)?;
        }
        self.registrar.unregister(action);
        if let Some(next) = &next
            && self.registrar.register(action, next).is_err()
        {
            self.restore(action, &saved, &mut states);
            return Err(HotkeyError::Unavailable);
        }
        let canonical = next.as_ref().map(ToString::to_string);
        let updated = self
            .settings
            .update(|settings| action.save(&mut settings.shortcuts, canonical))
            .await;
        let settings = match updated {
            Ok(settings) => settings,
            Err(error) => {
                self.registrar.unregister(action);
                self.restore(action, &saved, &mut states);
                return Err(error.into());
            }
        };
        let state = next.map_or(HotkeyState::Off, |_| HotkeyState::Active);
        states.insert(action, state);
        Ok(listing(&settings.shortcuts, &states))
    }

    /// Registers `text` for `action` unless it is invalid or `claimed`.
    fn claim(
        &self,
        action: HotkeyAction,
        text: &str,
        claimed: &mut Vec<(Accelerator, HotkeyAction)>,
    ) -> Result<(), HotkeyError> {
        let accelerator: Accelerator = text.parse()?;
        if let Some((_, owner)) = claimed.iter().find(|(taken, _)| *taken == accelerator) {
            return Err(HotkeyError::InUse(*owner));
        }
        self.registrar
            .register(action, &accelerator)
            .map_err(|_| HotkeyError::Unavailable)?;
        claimed.push((accelerator, action));
        Ok(())
    }

    /// Re-registers `action`'s saved hotkey after a failed change.
    fn restore(&self, action: HotkeyAction, saved: &ShortcutSettings, states: &mut States) {
        if states.get(&action) != Some(&HotkeyState::Active) {
            return;
        }
        let restored = action
            .saved(saved)
            .and_then(|text| text.parse::<Accelerator>().ok())
            .is_some_and(|accelerator| self.registrar.register(action, &accelerator).is_ok());
        if !restored {
            log::warn!(target: "hotkeys", "could not restore the {action:?} hotkey");
            states.insert(action, failed(&HotkeyError::Unavailable));
        }
    }
}

/// Refuses `next` when another action's saved hotkey is the same.
fn check_free(
    action: HotkeyAction,
    next: &Accelerator,
    saved: &ShortcutSettings,
) -> Result<(), HotkeyError> {
    let taken_by = HotkeyAction::ALL
        .into_iter()
        .filter(|other| *other != action)
        .find(|other| {
            other
                .saved(saved)
                .and_then(|text| text.parse::<Accelerator>().ok())
                .is_some_and(|theirs| theirs == *next)
        });
    taken_by.map_or(Ok(()), |other| Err(HotkeyError::InUse(other)))
}

fn failed(error: &HotkeyError) -> HotkeyState {
    HotkeyState::Failed {
        reason: error.to_string(),
    }
}

/// Every action with its saved value and state.
fn listing(saved: &ShortcutSettings, states: &States) -> Vec<Hotkey> {
    HotkeyAction::ALL
        .into_iter()
        .map(|action| Hotkey {
            action,
            label: action.label().to_owned(),
            accelerator: action.saved(saved).map(str::to_owned),
            default_accelerator: action.default_accelerator().to_owned(),
            state: states.get(&action).cloned().unwrap_or(HotkeyState::Off),
        })
        .collect()
}

impl std::fmt::Debug for HotkeyService {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("HotkeyService")
            .finish_non_exhaustive()
    }
}
