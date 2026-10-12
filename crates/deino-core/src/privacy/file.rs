//! The [`DatabaseFile`] port: the closed database file on disk.

use std::path::PathBuf;

use async_trait::async_trait;
use deino_db::{Conversion, DbError, FileFormat, Recovery};

/// The database file, while no connection has it open.
#[async_trait]
pub trait DatabaseFile: Send + Sync {
    /// Finishes or undoes a conversion a crash interrupted.
    fn recover(&self) -> Result<Recovery, DbError>;

    /// What is on disk now.
    fn format(&self) -> Result<FileFormat, DbError>;

    /// Converts the file, keeping the original until the copy is verified;
    /// on failure the file is unchanged.
    async fn convert(&self, conversion: Conversion<'_>) -> Result<(), DbError>;
}

/// The real file, through `deino_db`.
#[derive(Clone, Debug)]
pub struct OnDiskDatabase {
    path: PathBuf,
}

impl OnDiskDatabase {
    /// The database at `path`.
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

#[async_trait]
impl DatabaseFile for OnDiskDatabase {
    fn recover(&self) -> Result<Recovery, DbError> {
        deino_db::recover(&self.path)
    }

    fn format(&self) -> Result<FileFormat, DbError> {
        FileFormat::detect(&self.path)
    }

    async fn convert(&self, conversion: Conversion<'_>) -> Result<(), DbError> {
        deino_db::convert(&self.path, conversion).await
    }
}
