//! The speaker models bundled in the app (`speakers/` among its resources,
//! `frontend/src-tauri/resources/speakers/` in the repository, Git LFS),
//! made by `scripts/export-speaker-models.py`. Nothing is downloaded for
//! them: Community-1 is gated on Hugging Face, and everything runs on the
//! Mac (`docs/plans/python-speakers-port.md`).
//!
//! [`SEGMENTATION`](super::SEGMENTATION) and [`EMBEDDING`](super::EMBEDDING)
//! are bundled too, under the same names and pins, so the store can read
//! them from the app instead of downloading them.

/// One bundled file, pinned like the downloaded ones.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BundledFile {
    /// Its name in the bundled folder.
    pub file_name: &'static str,
    /// Its SHA-256, lowercase hex.
    pub sha256: &'static str,
    /// Its exact size.
    pub bytes: u64,
}

/// The bundled folder's name among the app's resources.
pub const FOLDER: &str = "speakers";

/// pyannote Community-1's segmentation model (CC-BY-4.0), revision
/// `3533c8c`: `input_values` [batch, 1, samples] → `logits` [batch, frames, 7].
pub const COMMUNITY_SEGMENTATION: BundledFile = BundledFile {
    file_name: "community-1-segmentation.onnx",
    sha256: "6b6d4b0382b1a975a7194de6f809a15c85a1fad768fa31756dc6814a70ee1208",
    bytes: 5_916_337,
};

/// Community-1's PLDA (CC-BY-4.0): `mu`, `tr`, `psi` as `NumPy` arrays.
pub const COMMUNITY_PLDA: BundledFile = BundledFile {
    file_name: "community-1-plda.npz",
    sha256: "9b77bcd840692710dd3496f62ecfeed8d8e5f002fd991b785079b244eab7d255",
    bytes: 133_852,
};

/// Community-1's x-vector transform (CC-BY-4.0): `mean1`, `mean2`, `lda`.
pub const COMMUNITY_XVEC_TRANSFORM: BundledFile = BundledFile {
    file_name: "community-1-xvec-transform.npz",
    sha256: "325f1ce8e48f7e55e9c8aa47e05d2766b7c48c4b25b8de8dd751e7a4cc5fbe8f",
    bytes: 134_376,
};

/// facebook/wav2vec2-base-960h (Apache-2.0), revision `22aad52`, `MatMul`
/// weights in int8: `input_values` [batch, samples] → `logits` [batch,
/// frames, 32], one frame per 20 ms.
pub const ALIGNER: BundledFile = BundledFile {
    file_name: "wav2vec2-base-960h-int8.onnx",
    sha256: "eb4d50a9c6b9291423e6b65f28024a6d5042d8dd1b5274fdaf8587b3ce06958c",
    bytes: 122_372_266,
};

/// Every bundled file, the downloaded models' copies included.
pub const FILES: [BundledFile; 6] = [
    COMMUNITY_SEGMENTATION,
    COMMUNITY_PLDA,
    COMMUNITY_XVEC_TRANSFORM,
    ALIGNER,
    as_bundled(super::SEGMENTATION),
    as_bundled(super::EMBEDDING),
];

const fn as_bundled(model: super::ModelFile) -> BundledFile {
    BundledFile {
        file_name: model.file_name,
        sha256: model.sha256,
        bytes: model.bytes,
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use sha2::{Digest, Sha256};

    use super::*;

    /// The repository's copy, when Git LFS has fetched it (a checkout
    /// without LFS has small pointer files, which this skips).
    #[test]
    fn the_bundled_files_match_their_pins() {
        let folder = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../frontend/src-tauri/resources")
            .join(FOLDER);
        for file in FILES {
            let path = folder.join(file.file_name);
            let Ok(metadata) = std::fs::metadata(&path) else {
                panic!("{} is not in the repository", file.file_name);
            };
            if metadata.len() != file.bytes {
                assert!(
                    metadata.len() < 1024,
                    "{} has the wrong size",
                    file.file_name
                );
                continue;
            }
            let digest = Sha256::digest(std::fs::read(&path).unwrap());
            assert_eq!(hex(&digest), file.sha256, "{}", file.file_name);
        }
    }

    fn hex(bytes: &[u8]) -> String {
        use std::fmt::Write;
        bytes.iter().fold(String::new(), |mut out, byte| {
            write!(out, "{byte:02x}").unwrap();
            out
        })
    }
}
