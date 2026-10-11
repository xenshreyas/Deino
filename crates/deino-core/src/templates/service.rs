//! [`TemplateService`]: the template library (Settings → Templates).
//!
//! List, read, create, edit, duplicate, delete, import and export. Built-ins
//! are read-only: editing or deleting one fails with
//! [`TemplateError::BuiltinIsReadOnly`], and "Duplicate" makes an editable copy.

use std::sync::Arc;

use deino_domain::{Clock, TemplateId};

use super::{
    LegacyTemplateFolder, MAX_NAME_CHARS, Template, TemplateContent, TemplateError, TemplateRepo,
    export_document, import_document,
};

/// What a copy's name gets.
const COPY_SUFFIX: &str = " (copy)";

/// Reads and edits templates.
pub struct TemplateService {
    repo: Arc<dyn TemplateRepo>,
    clock: Arc<dyn Clock>,
}

impl TemplateService {
    /// Creates the service over its ports.
    pub fn new(repo: Arc<dyn TemplateRepo>, clock: Arc<dyn Clock>) -> Self {
        Self { repo, clock }
    }

    /// Every template, by name.
    pub async fn list(&self) -> Result<Vec<Template>, TemplateError> {
        Ok(self.repo.list().await?)
    }

    /// The template with `id`.
    pub async fn get(&self, id: &TemplateId) -> Result<Template, TemplateError> {
        self.repo
            .get(id)
            .await?
            .ok_or_else(|| TemplateError::NotFound(id.clone()))
    }

    /// Validates `content` and saves it as a new template of the user's.
    pub async fn create(&self, content: TemplateContent) -> Result<Template, TemplateError> {
        let now = self.clock.now();
        let template = Template {
            id: TemplateId::generate(self.clock.as_ref()),
            is_builtin: false,
            content: content.validated()?,
            created_at: now,
            updated_at: now,
        };
        self.repo.insert(&template).await?;
        log::debug!(target: "templates", "created template {}", template.id);
        Ok(template)
    }

    /// Replaces the content of one of the user's templates.
    pub async fn update(
        &self,
        id: &TemplateId,
        content: TemplateContent,
    ) -> Result<Template, TemplateError> {
        let mut template = self.editable(id).await?;
        template.content = content.validated()?;
        template.updated_at = self.clock.now();
        if !self.repo.update(&template).await? {
            return Err(TemplateError::NotFound(id.clone()));
        }
        Ok(template)
    }

    /// An editable copy of any template, named "<name> (copy)".
    pub async fn duplicate(&self, id: &TemplateId) -> Result<Template, TemplateError> {
        let mut content = self.get(id).await?.content;
        content.name = copy_name(&content.name);
        self.create(content).await
    }

    /// Deletes one of the user's templates; meetings using it go back to
    /// automatic selection.
    pub async fn delete(&self, id: &TemplateId) -> Result<(), TemplateError> {
        self.editable(id).await?;
        if !self.repo.delete(id).await? {
            return Err(TemplateError::NotFound(id.clone()));
        }
        Ok(())
    }

    /// The template as a JSON file the user can share.
    pub async fn export(&self, id: &TemplateId) -> Result<String, TemplateError> {
        export_document(&self.get(id).await?.content)
    }

    /// Saves a template file (current or legacy format) as a new template.
    pub async fn import(&self, json: &str) -> Result<Template, TemplateError> {
        self.create(import_document(json)?).await
    }

    /// Imports every pre-NOT-04 custom template file in `folder` and archives
    /// it. Returns how many were imported; a file that fails stays in place.
    pub async fn import_legacy(
        &self,
        folder: &LegacyTemplateFolder,
    ) -> Result<usize, TemplateError> {
        let mut imported = 0;
        for file in folder.templates()? {
            match self.create(file.content.clone()).await {
                Ok(_) => {
                    folder.archive(&file)?;
                    imported += 1;
                }
                Err(error) => {
                    log::warn!(target: "templates", "skipped a legacy template file: {error}");
                }
            }
        }
        Ok(imported)
    }

    /// The template with `id`, if the user may change it.
    async fn editable(&self, id: &TemplateId) -> Result<Template, TemplateError> {
        let template = self.get(id).await?;
        if template.is_builtin {
            return Err(TemplateError::BuiltinIsReadOnly(id.clone()));
        }
        Ok(template)
    }
}

/// `name (copy)`, shortened so it stays within [`MAX_NAME_CHARS`].
fn copy_name(name: &str) -> String {
    let room = MAX_NAME_CHARS - COPY_SUFFIX.chars().count();
    let base: String = name.chars().take(room).collect();
    format!("{}{COPY_SUFFIX}", base.trim_end())
}
