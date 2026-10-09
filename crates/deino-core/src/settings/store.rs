//! The settings storage port and its JSON-file adapter.
//!
//! [`JsonFileSettingsStore`] keeps `settings.json` in the app's config
//! directory. A save never leaves a half-written file: it writes a sibling
//! temp file, `fsync`s it, renames it over the old file (atomic on one
//! volume) and `fsync`s the directory so the rename itself survives a crash.

use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use async_trait::async_trait;

use super::{Settings, SettingsError};

/// Where settings are kept.
#[async_trait]
pub trait SettingsStore: Send + Sync {
    /// The saved settings, upgraded to the current version; `None` when
    /// nothing has been saved yet.
    async fn load(&self) -> Result<Option<Settings>, SettingsError>;

    /// Replaces the saved settings with `settings`, atomically.
    async fn save(&self, settings: &Settings) -> Result<(), SettingsError>;
}

/// [`SettingsStore`] over one pretty-printed JSON file.
#[derive(Clone, Debug)]
pub struct JsonFileSettingsStore {
    path: PathBuf,
}

impl JsonFileSettingsStore {
    /// The file the app uses, inside its config directory.
    pub const FILE_NAME: &'static str = "settings.json";

    /// A store over `path`; its directory is created on the first save.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Where an unreadable settings file is moved, so it can be inspected
    /// instead of being overwritten by the next save.
    fn quarantine_path(&self) -> PathBuf {
        self.path.with_extension("unreadable.json")
    }
}

#[async_trait]
impl SettingsStore for JsonFileSettingsStore {
    async fn load(&self) -> Result<Option<Settings>, SettingsError> {
        let store = self.clone();
        run_blocking(move || store.load_blocking()).await
    }

    async fn save(&self, settings: &Settings) -> Result<(), SettingsError> {
        let mut bytes = serde_json::to_vec_pretty(settings).map_err(SettingsError::Encode)?;
        bytes.push(b'\n');
        let path = self.path.clone();
        run_blocking(move || write_atomically(&path, &bytes).map_err(SettingsError::Write)).await
    }
}

impl JsonFileSettingsStore {
    fn load_blocking(&self) -> Result<Option<Settings>, SettingsError> {
        let bytes = match fs::read(&self.path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(SettingsError::Read(error)),
        };
        match serde_json::from_slice::<Settings>(&bytes) {
            Ok(settings) => Ok(Some(settings.upgraded())),
            Err(error) => {
                // Starting from defaults beats refusing to start; the old
                // file is kept next to the new one for recovery by hand.
                log::warn!(target: "settings", "settings file is unreadable ({error}); starting from defaults");
                fs::rename(&self.path, self.quarantine_path()).map_err(SettingsError::Read)?;
                Ok(None)
            }
        }
    }
}

/// Writes `bytes` to `path` so that readers see the old file or the new
/// one, never a mix, even across a crash or power loss.
fn write_atomically(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let directory = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(directory)?;
    let temp = path.with_extension("json.tmp");
    let mut file = File::create(&temp)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);
    fs::rename(&temp, path)?;
    sync_directory(directory)
}

/// Makes a rename inside `directory` durable.
#[cfg(unix)]
fn sync_directory(directory: &Path) -> io::Result<()> {
    File::open(directory)?.sync_all()
}

/// Windows cannot open a directory as a file; its renames are already
/// journaled by NTFS.
#[cfg(not(unix))]
fn sync_directory(_directory: &Path) -> io::Result<()> {
    Ok(())
}

/// Runs blocking file I/O off the async runtime.
async fn run_blocking<T, F>(work: F) -> Result<T, SettingsError>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, SettingsError> + Send + 'static,
{
    tokio::task::spawn_blocking(work)
        .await
        .map_err(|join| SettingsError::Write(io::Error::other(join)))?
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Theme;

    fn store_in(directory: &tempfile::TempDir) -> JsonFileSettingsStore {
        JsonFileSettingsStore::new(directory.path().join("config").join("settings.json"))
    }

    #[tokio::test]
    async fn nothing_saved_loads_as_none() {
        let directory = tempfile::tempdir().unwrap();

        assert_eq!(store_in(&directory).load().await.unwrap(), None);
    }

    #[tokio::test]
    async fn saved_settings_load_back_and_leave_no_temp_file() {
        let directory = tempfile::tempdir().unwrap();
        let store = store_in(&directory);
        let mut settings = Settings::default();
        settings.ui.theme = Theme::Dark;

        store.save(&settings).await.unwrap();

        assert_eq!(store.load().await.unwrap(), Some(settings));
        let temp = store.path.with_extension("json.tmp");
        assert!(!temp.exists());
    }

    #[tokio::test]
    async fn an_unreadable_file_is_set_aside_and_defaults_are_used() {
        let directory = tempfile::tempdir().unwrap();
        let store = store_in(&directory);
        fs::create_dir_all(store.path.parent().unwrap()).unwrap();
        fs::write(&store.path, "{ not json").unwrap();

        assert_eq!(store.load().await.unwrap(), None);
        assert_eq!(
            fs::read_to_string(store.quarantine_path()).unwrap(),
            "{ not json"
        );
    }
}
