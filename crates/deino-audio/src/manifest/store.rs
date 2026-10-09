//! Reading and atomically writing `recording.json`.

use std::io;
use std::path::{Path, PathBuf};

use deino_domain::{ErrorClass, ErrorCode};
use serde::Deserialize;

use super::{MANIFEST_FILE, ManifestVersion, RecordingManifest};
use crate::durability::write_atomic;

/// Why the manifest could not be read or written.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ManifestError {
    /// There is no `recording.json` in the directory.
    #[error("the recording has no manifest")]
    Missing,
    /// Reading or writing the file failed.
    #[error("could not read or write the recording manifest")]
    Io(#[from] io::Error),
    /// The file is not a valid manifest.
    #[error("the recording manifest is damaged")]
    Malformed(#[from] serde_json::Error),
    /// The file was written by a newer version of Deino.
    #[error("the recording manifest has version {0}, newer than this app supports")]
    UnsupportedVersion(u32),
}

impl ErrorClass for ManifestError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::Missing => ErrorCode::NotFound,
            Self::UnsupportedVersion(_) => ErrorCode::Unavailable,
            Self::Io(_) | Self::Malformed(_) => ErrorCode::Internal,
        }
    }
}

/// Just the version, read before the rest so a newer format fails clearly.
#[derive(Deserialize)]
struct VersionProbe {
    version: ManifestVersion,
}

/// The manifest of one recording directory.
#[derive(Clone, Debug)]
pub struct ManifestStore {
    path: PathBuf,
}

impl ManifestStore {
    /// The store for the recording in `dir`.
    pub fn new(dir: &Path) -> Self {
        Self {
            path: dir.join(MANIFEST_FILE),
        }
    }

    /// The manifest file's path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Reads and checks the manifest.
    pub fn read(&self) -> Result<RecordingManifest, ManifestError> {
        let bytes = match std::fs::read(&self.path) {
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Err(ManifestError::Missing);
            }
            read => read?,
        };
        let probe: VersionProbe = serde_json::from_slice(&bytes)?;
        if probe.version > ManifestVersion::CURRENT {
            return Err(ManifestError::UnsupportedVersion(probe.version.0));
        }
        Ok(serde_json::from_slice(&bytes)?)
    }

    /// Replaces the manifest atomically and durably (temp file, full fsync,
    /// rename, directory fsync), so a crash leaves the old or the new one.
    pub fn write(&self, manifest: &RecordingManifest) -> Result<(), ManifestError> {
        let bytes = serde_json::to_vec_pretty(manifest)?;
        write_atomic(&self.path, &bytes)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};

    use super::*;
    use crate::manifest::RecordingStatus;
    use crate::units::HostTime;

    fn manifest() -> RecordingManifest {
        let started_at = Utc.with_ymd_and_hms(2026, 10, 9, 14, 0, 0).unwrap();
        RecordingManifest::start(HostTime(77), started_at)
    }

    #[test]
    fn a_written_manifest_reads_back_equal() {
        let dir = tempfile::tempdir().unwrap();
        let store = ManifestStore::new(dir.path());
        let mut written = manifest();
        written.status = RecordingStatus::Closed;

        store.write(&written).unwrap();

        assert_eq!(store.read().unwrap(), written);
    }

    #[test]
    fn a_missing_manifest_is_reported_as_missing() {
        let dir = tempfile::tempdir().unwrap();
        let error = ManifestStore::new(dir.path()).read().unwrap_err();
        assert!(matches!(error, ManifestError::Missing));
        assert_eq!(error.code(), ErrorCode::NotFound);
    }

    #[test]
    fn a_newer_version_is_refused_before_parsing_the_rest() {
        let dir = tempfile::tempdir().unwrap();
        let store = ManifestStore::new(dir.path());
        std::fs::write(store.path(), r#"{"version": 2, "shape": "unknown"}"#).unwrap();

        let error = store.read().unwrap_err();

        assert!(matches!(error, ManifestError::UnsupportedVersion(2)));
    }

    #[test]
    fn a_damaged_manifest_is_malformed() {
        let dir = tempfile::tempdir().unwrap();
        let store = ManifestStore::new(dir.path());
        std::fs::write(store.path(), b"{\"version\": 1, \"sample_ra").unwrap();

        assert!(matches!(
            store.read().unwrap_err(),
            ManifestError::Malformed(_)
        ));
    }
}
