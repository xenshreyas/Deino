//! The bundled model catalog: `catalog.json`, compiled into the binary.
//!
//! Every entry is pinned to a repository revision, and every file to its
//! exact size and SHA-256 (copied from Handy's catalog, MIT). Downloads are
//! accepted only when both match, so the hash, not the server, is trusted.

use serde::Deserialize;

use super::{CatalogFile, CatalogModel, ModelError, ModelFileId, Quant};

/// The catalog JSON.
const CATALOG_JSON: &str = include_str!("catalog.json");

/// The catalog as stored.
#[derive(Deserialize)]
struct RawCatalog {
    models: Vec<RawModel>,
}

#[derive(Deserialize)]
struct RawModel {
    repo: String,
    revision: String,
    name: String,
    license: String,
    default_quant: Quant,
    files: Vec<RawFile>,
}

#[derive(Deserialize)]
struct RawFile {
    filename: String,
    quant: Quant,
    size_bytes: u64,
    sha256: String,
}

/// The models Deino can download.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Catalog {
    models: Vec<CatalogModel>,
}

impl Catalog {
    /// The catalog compiled into the app.
    pub fn bundled() -> Result<Self, ModelError> {
        Self::parse(CATALOG_JSON)
    }

    /// Parses and validates catalog JSON.
    pub fn parse(json: &str) -> Result<Self, ModelError> {
        let raw: RawCatalog = serde_json::from_str(json)
            .map_err(|error| ModelError::InvalidCatalog(error.to_string()))?;
        let models = raw
            .models
            .into_iter()
            .map(model)
            .collect::<Result<_, _>>()?;
        Ok(Self { models })
    }

    /// The live transcription model (Parakeet Unified EN 0.6B), the first entry.
    pub fn live_model(&self) -> Result<&CatalogModel, ModelError> {
        self.models
            .first()
            .ok_or_else(|| ModelError::InvalidCatalog("the catalog is empty".into()))
    }

    /// The model and file with ID `id`.
    pub fn find(&self, id: &ModelFileId) -> Option<(&CatalogModel, &CatalogFile)> {
        self.models.iter().find_map(|model| {
            let file = model.files.iter().find(|file| &file.id == id)?;
            Some((model, file))
        })
    }
}

fn model(raw: RawModel) -> Result<CatalogModel, ModelError> {
    let files = raw
        .files
        .into_iter()
        .map(file)
        .collect::<Result<Vec<_>, _>>()?;
    if !files.iter().any(|file| file.quant == raw.default_quant) {
        return Err(ModelError::InvalidCatalog(format!(
            "{} lacks its default quant",
            raw.name
        )));
    }
    Ok(CatalogModel {
        name: raw.name,
        repo: raw.repo,
        revision: raw.revision,
        license: raw.license,
        default_quant: raw.default_quant,
        files,
    })
}

fn file(raw: RawFile) -> Result<CatalogFile, ModelError> {
    Ok(CatalogFile {
        id: ModelFileId::of_filename(&raw.filename),
        sha256: raw.sha256.parse()?,
        filename: raw.filename,
        quant: raw.quant,
        size_bytes: raw.size_bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bundled_catalog_pins_all_six_parakeet_unified_quants() {
        let catalog = Catalog::bundled().unwrap();
        let model = catalog.live_model().unwrap();

        assert_eq!(model.name, "Parakeet Unified EN 0.6B");
        assert_eq!(model.revision, "7e948f21b7bdbac698d3318db9d350f1096f3b6c");
        assert_eq!(model.files.len(), 6);
        let q8 = model.file(Quant::Q8).unwrap();
        assert_eq!(q8.size_bytes, 731_357_568);
        assert_eq!(
            q8.sha256.to_string(),
            "4b50b6dd862bf6e346929aaf4f5eaacec003bfa3f56462d6c874b41ef2f38795"
        );
        assert_eq!(
            model.url(q8),
            "https://huggingface.co/handy-computer/parakeet-unified-en-0.6b-gguf/resolve/7e948f21b7bdbac698d3318db9d350f1096f3b6c/parakeet-unified-en-0.6b-Q8_0.gguf"
        );
    }

    #[test]
    fn files_are_found_by_id() {
        let catalog = Catalog::bundled().unwrap();
        let id = ModelFileId::of_filename("parakeet-unified-en-0.6b-Q5_K_M.gguf");

        let (_, file) = catalog.find(&id).unwrap();

        assert_eq!(file.quant, Quant::Q5KM);
    }

    #[test]
    fn a_catalog_without_its_default_quant_is_rejected() {
        let json = r#"{"models":[{"repo":"r","revision":"v","name":"n","license":"l",
            "default_quant":"Q8_0","files":[]}]}"#;

        assert!(matches!(
            Catalog::parse(json),
            Err(ModelError::InvalidCatalog(_))
        ));
    }
}
