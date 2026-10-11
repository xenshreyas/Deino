//! Where the database is: `--db`, else `DEINO_DB`, else the app's own file.
//!
//! The app finds `deino.sqlite` in Tauri's app data dir, which on every
//! platform is the user's data directory (`dirs::data_dir()`, on macOS
//! `~/Library/Application Support`) joined with the bundle identifier. This
//! is the one place outside the app that repeats that rule; a test keeps
//! [`APP_IDENTIFIER`] equal to `tauri.conf.json`'s.

use std::path::PathBuf;

/// The app's bundle identifier, which names its data folder.
pub const APP_IDENTIFIER: &str = "com.xenshreyas.deino";
/// The database file inside the data folder (the app's `DB_FILE_NAME`).
pub const DB_FILE_NAME: &str = "deino.sqlite";
/// The environment variable that points at another database.
pub const DB_ENV: &str = "DEINO_DB";

/// The database the app itself uses, or `None` when the platform has no
/// user data directory.
pub fn app_database() -> Option<PathBuf> {
    dirs::data_dir().map(|data| data.join(APP_IDENTIFIER).join(DB_FILE_NAME))
}

/// `explicit` (from `--db` or `DEINO_DB`), else the app's database.
pub fn resolve(explicit: Option<PathBuf>) -> Option<PathBuf> {
    explicit.or_else(app_database)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_identifier_is_the_apps() {
        let config = include_str!("../../../frontend/src-tauri/tauri.conf.json");
        let config: serde_json::Value = serde_json::from_str(config).unwrap();

        assert_eq!(config["identifier"], APP_IDENTIFIER);
    }

    #[test]
    fn an_explicit_path_wins() {
        let explicit = PathBuf::from("/tmp/other.sqlite");

        assert_eq!(resolve(Some(explicit.clone())), Some(explicit));
    }

    #[test]
    fn the_default_is_the_app_data_file() {
        let path = resolve(None).unwrap();

        assert!(path.ends_with("com.xenshreyas.deino/deino.sqlite"));
    }
}
