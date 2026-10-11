//! The model catalog's types and what the UI sees of a model file.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use super::ModelError;

/// Macs with at least this much memory run two `Q8_0` lanes (about 2 GB in
/// two worker processes); smaller ones default to `Q5_K_M`, which saves
/// about 180 MB per lane at the same accuracy (STT spike, §7 and §9).
const LARGE_MEMORY_BYTES: u64 = 32 * 1024 * 1024 * 1024;

/// How a model's weights are quantized: smaller is lighter, larger is closer
/// to the original.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
pub enum Quant {
    /// 4-bit, the smallest.
    #[serde(rename = "Q4_K_M")]
    Q4KM,
    /// 5-bit, the default on Macs with less than 32 GB.
    #[serde(rename = "Q5_K_M")]
    Q5KM,
    /// 6-bit.
    #[serde(rename = "Q6_K")]
    Q6K,
    /// 8-bit, the default.
    #[serde(rename = "Q8_0")]
    Q8,
    /// 16-bit floats.
    F16,
    /// 32-bit floats, the original weights.
    F32,
}

impl Quant {
    /// The quant to use on a Mac with `memory_bytes` of RAM: `Q8_0` from
    /// 32 GB, `Q5_K_M` below.
    pub fn default_for_memory(memory_bytes: u64) -> Self {
        if memory_bytes >= LARGE_MEMORY_BYTES {
            Self::Q8
        } else {
            Self::Q5KM
        }
    }
}

/// A model file's ID: its file name without the extension, such as
/// `parakeet-unified-en-0.6b-Q8_0`. Stored as the selected model.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(transparent)]
pub struct ModelFileId(String);

impl ModelFileId {
    /// An ID as stored, such as the selected model in the transcript config.
    /// Unknown IDs are rejected when the catalog is asked for them.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// The ID of `filename`.
    pub fn of_filename(filename: &str) -> Self {
        let stem = filename.rsplit_once('.').map_or(filename, |(stem, _)| stem);
        Self(stem.to_owned())
    }

    /// The ID as text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ModelFileId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// A SHA-256 digest, the trust anchor for a downloaded file.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Sha256Digest([u8; 32]);

impl Sha256Digest {
    /// Wraps raw digest bytes.
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

impl FromStr for Sha256Digest {
    type Err = ModelError;

    fn from_str(hex: &str) -> Result<Self, Self::Err> {
        let invalid = || ModelError::InvalidCatalog(format!("bad sha256 {hex:?}"));
        if hex.len() != 64 {
            return Err(invalid());
        }
        let mut bytes = [0_u8; 32];
        for (byte, pair) in bytes.iter_mut().zip(hex.as_bytes().chunks(2)) {
            let pair = std::str::from_utf8(pair).map_err(|_| invalid())?;
            *byte = u8::from_str_radix(pair, 16).map_err(|_| invalid())?;
        }
        Ok(Self(bytes))
    }
}

impl fmt::Display for Sha256Digest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0
            .iter()
            .try_for_each(|byte| write!(formatter, "{byte:02x}"))
    }
}

impl fmt::Debug for Sha256Digest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "Sha256Digest({self})")
    }
}

/// One downloadable file of a catalog model.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CatalogFile {
    /// The file's ID.
    pub id: ModelFileId,
    /// Its name in the repository and on disk.
    pub filename: String,
    /// Its quantization.
    pub quant: Quant,
    /// Its exact size.
    pub size_bytes: u64,
    /// Its exact content.
    pub sha256: Sha256Digest,
}

/// What a catalog model is for; each [`ModelService`](super::ModelService)
/// serves one purpose.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelPurpose {
    /// Speech to text (Parakeet Unified), the default.
    #[default]
    Transcription,
    /// Text to vectors for semantic search (AI-01).
    Embedding,
}

/// A model in the catalog, pinned to one repository revision.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CatalogModel {
    /// The name the UI shows.
    pub name: String,
    /// What it is for.
    pub purpose: ModelPurpose,
    /// The Hugging Face repository.
    pub repo: String,
    /// The pinned revision (a commit hash).
    pub revision: String,
    /// The weights' licence.
    pub license: String,
    /// The quant used when the Mac has plenty of memory.
    pub default_quant: Quant,
    /// Every quant, smallest first.
    pub files: Vec<CatalogFile>,
}

impl CatalogModel {
    /// The file for `quant`, if the model ships it.
    pub fn file(&self, quant: Quant) -> Option<&CatalogFile> {
        self.files.iter().find(|file| file.quant == quant)
    }

    /// The download URL of `file`, pinned to the catalog revision.
    pub fn url(&self, file: &CatalogFile) -> String {
        format!(
            "https://huggingface.co/{}/resolve/{}/{}",
            self.repo, self.revision, file.filename
        )
    }
}

/// Where a model file stands on this Mac.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ModelState {
    /// Not downloaded.
    Missing,
    /// A download stopped part-way; it resumes from here.
    Partial {
        /// Bytes already on disk.
        #[serde(rename = "downloadedBytes")]
        downloaded_bytes: u64,
    },
    /// Downloading now.
    Downloading {
        /// Bytes received so far.
        #[serde(rename = "downloadedBytes")]
        downloaded_bytes: u64,
    },
    /// Downloaded and verified.
    Ready,
}

/// A model file as the model manager lists it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ModelFileView {
    /// The file's ID, used to download, delete and select it.
    pub id: ModelFileId,
    /// The model's name.
    pub name: String,
    /// Its quantization.
    pub quant: Quant,
    /// Its download size.
    pub size_bytes: u64,
    /// Whether it is on this Mac.
    pub state: ModelState,
    /// Whether it is the right quant for this Mac's memory.
    pub recommended: bool,
}

/// How far a download has got.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgress {
    /// The file being downloaded.
    pub id: ModelFileId,
    /// Bytes on disk so far.
    pub downloaded_bytes: u64,
    /// The file's full size.
    pub total_bytes: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn macs_with_32_gigabytes_get_q8_and_smaller_ones_get_q5() {
        assert_eq!(Quant::default_for_memory(64 << 30), Quant::Q8);
        assert_eq!(Quant::default_for_memory(32 << 30), Quant::Q8);
        assert_eq!(Quant::default_for_memory(24 << 30), Quant::Q5KM);
        assert_eq!(Quant::default_for_memory(8 << 30), Quant::Q5KM);
    }

    #[test]
    fn digests_round_trip_through_hex_and_reject_garbage() {
        let hex = "4b50b6dd862bf6e346929aaf4f5eaacec003bfa3f56462d6c874b41ef2f38795";

        assert_eq!(hex.parse::<Sha256Digest>().unwrap().to_string(), hex);
        assert!("4b50".parse::<Sha256Digest>().is_err());
        assert!("zz".repeat(32).parse::<Sha256Digest>().is_err());
    }

    #[test]
    fn a_file_id_is_its_name_without_the_extension() {
        let id = ModelFileId::of_filename("parakeet-unified-en-0.6b-Q8_0.gguf");

        assert_eq!(id.as_str(), "parakeet-unified-en-0.6b-Q8_0");
    }

    #[test]
    fn quants_serialize_with_their_gguf_names() {
        assert_eq!(serde_json::to_string(&Quant::Q4KM).unwrap(), r#""Q4_K_M""#);
        assert_eq!(serde_json::to_string(&Quant::Q8).unwrap(), r#""Q8_0""#);
    }
}
