//! The two diarization models: where they come from, their pins, and the
//! store that downloads and verifies them.
//!
//! anarlog ships its models as committed blobs with no download source, so
//! Deino fetches them on demand from their publishers, pinned by revision,
//! byte size and SHA-256 (`docs/adr/0019-local-speaker-diarization.md`),
//! through the same verified downloader and `ByteSource` port as the
//! transcription models (`deino_core::models`):
//!
//! - **pyannote segmentation-3.0** (MIT, © CNRS): the onnx-community export
//!   on Hugging Face, byte-identical to anarlog's bundled copy.
//! - **WeSpeaker ResNet34-LM** (CC-BY-4.0, © the WeSpeaker authors): the
//!   official export in the `Wespeaker` Hugging Face repository. It is a
//!   different file from anarlog's, but the same network: its embeddings
//!   match anarlog's at cosine similarity ≥ 0.999 on the fixtures (the
//!   `tests/model_parity.rs`). Deino computes the filterbank and its
//!   per-utterance mean normalisation itself, so the ONNX metadata sherpa-onnx
//!   reads (`feature_normalize_type`) plays no part.

pub mod store;

pub use store::{ModelStore, StoreError};

/// One pinned model file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ModelFile {
    /// The name it is stored under in the model store.
    pub file_name: &'static str,
    /// Where it is downloaded from (a revision-pinned URL).
    pub url: &'static str,
    /// Its SHA-256, lowercase hex.
    pub sha256: &'static str,
    /// Its exact size.
    pub bytes: u64,
}

/// pyannote segmentation-3.0, onnx-community export at revision `733a93b`.
pub const SEGMENTATION: ModelFile = ModelFile {
    file_name: "pyannote-segmentation-3.0.onnx",
    url: "https://huggingface.co/onnx-community/pyannote-segmentation-3.0/resolve/733a93b6473d019a773298e08cefa686894b1854/onnx/model.onnx",
    sha256: "057ee564753071c0b09b5b611648b50ac188d50846bff5f01e9f7bbf1591ea25",
    bytes: 5_986_908,
};

/// WeSpeaker ResNet34-LM (VoxCeleb), the official ONNX export at revision
/// `f0c48c2`.
pub const EMBEDDING: ModelFile = ModelFile {
    file_name: "wespeaker-voxceleb-resnet34-lm.onnx",
    url: "https://huggingface.co/Wespeaker/wespeaker-voxceleb-resnet34-LM/resolve/f0c48c298fd835726c27956a5d617bad7115627e/voxceleb_resnet34_LM.onnx",
    sha256: "7bb2f06e9df17cdf1ef14ee8a15ab08ed28e8d0ef5054ee135741560df2ec068",
    bytes: 26_530_309,
};

/// The pair of models one diarizer loads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Catalog {
    /// The segmentation model.
    pub segmentation: ModelFile,
    /// The speaker-embedding model.
    pub embedding: ModelFile,
}

impl Catalog {
    /// Both files.
    pub const fn files(&self) -> [ModelFile; 2] {
        [self.segmentation, self.embedding]
    }
}

/// The models Deino ships with.
pub const CATALOG: Catalog = Catalog {
    segmentation: SEGMENTATION,
    embedding: EMBEDDING,
};

/// The name stored next to every centroid and voiceprint, so vectors from
/// different embedding models are never compared.
pub const EMBEDDING_MODEL_ID: &str = "wespeaker-voxceleb-resnet34-lm@7bb2f06e";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_pin_is_a_lowercase_sha256_and_an_https_url() {
        for model in CATALOG.files() {
            assert_eq!(model.sha256.len(), 64, "{}", model.file_name);
            assert!(
                model
                    .sha256
                    .chars()
                    .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
            );
            assert!(
                model.url.starts_with("https://huggingface.co/"),
                "{}",
                model.url
            );
            assert!(model.bytes > 1_000_000);
        }
    }

    #[test]
    fn the_model_id_names_the_pinned_embedding_file() {
        assert!(EMBEDDING_MODEL_ID.ends_with(&EMBEDDING.sha256[..8]));
    }
}
