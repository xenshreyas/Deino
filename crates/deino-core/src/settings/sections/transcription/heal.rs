//! The engine self-heal: a configured engine whose model is not on this Mac
//! falls back to an installed Parakeet Unified file.
//!
//! An older build could leave `settings.json` pointing at an engine whose
//! model was never downloaded (Parakeet TDT, say) while Parakeet Unified is
//! installed; every recording then failed to start. The app runs
//! [`healed_transcription`] once at startup over its model folders and saves
//! the result. The rule is pure: [`InstalledModels`] is the port the app
//! implements over its model stores, and tests use a fake.

use super::{TranscriptionEngine, TranscriptionSettings};

/// What the model folders hold, as the heal rule needs it.
pub trait InstalledModels {
    /// Whether `engine`'s model `model` is installed and usable. `model` is
    /// the setting's value (empty means the engine's default); a value the
    /// engine cannot resolve to a file counts as not installed.
    fn is_installed(&self, engine: TranscriptionEngine, model: &str) -> bool;

    /// An installed Parakeet Unified file's ID, preferring the quant this Mac
    /// suits; `None` when no Parakeet Unified file is installed.
    fn installed_unified(&self) -> Option<String>;
}

/// The settings to save instead of `current`, or `None` to keep them.
///
/// - The configured engine's model is installed: keep it.
/// - It is not, but a Parakeet Unified file is: switch to Parakeet Unified
///   with that file (the capture modes are kept).
/// - Neither: keep it, so Settings → Transcription offers the download.
pub fn healed_transcription(
    current: &TranscriptionSettings,
    installed: &dyn InstalledModels,
) -> Option<TranscriptionSettings> {
    if installed.is_installed(current.engine, &current.model) {
        return None;
    }
    let model = installed.installed_unified()?;
    let next = TranscriptionSettings {
        engine: TranscriptionEngine::ParakeetUnified,
        model,
        ..current.clone()
    };
    (next != *current).then_some(next)
}

#[cfg(test)]
mod tests {
    use super::*;

    const UNIFIED: &str = "parakeet-unified-en-0.6b-Q8_0";
    const TDT: &str = "parakeet-tdt-0.6b-v3-int8";

    /// A fake model catalog: which (engine, model) pairs are installed.
    #[derive(Default)]
    struct FakeCatalog {
        installed: Vec<(TranscriptionEngine, &'static str)>,
    }

    impl InstalledModels for FakeCatalog {
        fn is_installed(&self, engine: TranscriptionEngine, model: &str) -> bool {
            self.installed.contains(&(engine, model))
        }

        fn installed_unified(&self) -> Option<String> {
            self.installed
                .iter()
                .find(|(engine, _)| *engine == TranscriptionEngine::ParakeetUnified)
                .map(|(_, model)| (*model).to_owned())
        }
    }

    fn tdt() -> TranscriptionSettings {
        TranscriptionSettings {
            engine: TranscriptionEngine::Parakeet,
            model: TDT.to_owned(),
            live_transcription: false,
            ..TranscriptionSettings::default()
        }
    }

    #[test]
    fn a_missing_tdt_model_switches_to_the_installed_unified_file() {
        let catalog = FakeCatalog {
            installed: vec![(TranscriptionEngine::ParakeetUnified, UNIFIED)],
        };

        let healed = healed_transcription(&tdt(), &catalog).unwrap();

        assert_eq!(healed.engine, TranscriptionEngine::ParakeetUnified);
        assert_eq!(healed.model, UNIFIED);
        assert!(!healed.live_transcription, "the capture modes are kept");
    }

    #[test]
    fn an_installed_tdt_model_stays() {
        let catalog = FakeCatalog {
            installed: vec![
                (TranscriptionEngine::Parakeet, TDT),
                (TranscriptionEngine::ParakeetUnified, UNIFIED),
            ],
        };

        assert_eq!(healed_transcription(&tdt(), &catalog), None);
    }

    #[test]
    fn with_neither_installed_nothing_changes() {
        assert_eq!(healed_transcription(&tdt(), &FakeCatalog::default()), None);
    }

    #[test]
    fn an_unresolvable_unified_model_heals_to_the_installed_file() {
        let catalog = FakeCatalog {
            installed: vec![(TranscriptionEngine::ParakeetUnified, UNIFIED)],
        };
        let unknown = TranscriptionSettings {
            model: "no-such-model".to_owned(),
            ..TranscriptionSettings::default()
        };

        let healed = healed_transcription(&unknown, &catalog).unwrap();

        assert_eq!(healed.model, UNIFIED);
    }
}
