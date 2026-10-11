//! [`FilterOptions`]: the choices the Action Items filter bar and the
//! assignee picker offer, read from the people, companies, tags and folders
//! the user has. Names only; each feature's own page edits them.

use std::sync::Arc;

use deino_db::DbError;
use deino_domain::{CompanyId, FolderId, PaletteColor, PersonId, TagId};
use serde::Serialize;

use crate::companies::CompanyRepo;
use crate::folders::FolderRepo;
use crate::people::PersonRepo;
use crate::tags::TagRepo;

/// A person to assign or filter by.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PersonOption {
    /// The person.
    pub id: PersonId,
    /// Their name.
    pub name: String,
    /// Their address, to tell two people of the same name apart.
    pub email: Option<String>,
    /// True for the user.
    pub is_me: bool,
}

/// A company to filter by.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CompanyOption {
    /// The company.
    pub id: CompanyId,
    /// Its name.
    pub name: String,
}

/// A tag to filter by.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TagOption {
    /// The tag.
    pub id: TagId,
    /// Its name.
    pub name: String,
    /// Its colour slot.
    pub color: PaletteColor,
}

/// A folder to filter by.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct FolderOption {
    /// The folder.
    pub id: FolderId,
    /// Its name.
    pub name: String,
    /// The folder it sits in, for showing the tree.
    pub parent_id: Option<FolderId>,
}

/// Everything the filter bar can filter by.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct FilterOptions {
    /// People, the user first, then by name.
    pub people: Vec<PersonOption>,
    /// Companies, by name.
    pub companies: Vec<CompanyOption>,
    /// Tags, by name.
    pub tags: Vec<TagOption>,
    /// Folders, by name.
    pub folders: Vec<FolderOption>,
}

/// The repositories the options come from.
#[derive(Clone)]
pub struct OptionSources {
    /// People.
    pub people: Arc<dyn PersonRepo>,
    /// Companies.
    pub companies: Arc<dyn CompanyRepo>,
    /// Tags.
    pub tags: Arc<dyn TagRepo>,
    /// Folders.
    pub folders: Arc<dyn FolderRepo>,
}

impl OptionSources {
    /// Reads every option.
    pub async fn load(&self) -> Result<FilterOptions, DbError> {
        let mut people: Vec<PersonOption> = self
            .people
            .list()
            .await?
            .into_iter()
            .map(|person| PersonOption {
                id: person.id,
                name: person.name,
                email: person.email.map(|email| email.as_str().to_owned()),
                is_me: person.is_me,
            })
            .collect();
        // Stable: the rest keep the repository's name order.
        people.sort_by_key(|person| !person.is_me);
        Ok(FilterOptions {
            people,
            companies: self.companies().await?,
            tags: self.tags().await?,
            folders: self.folders().await?,
        })
    }

    async fn companies(&self) -> Result<Vec<CompanyOption>, DbError> {
        let companies = self.companies.list().await?;
        Ok(companies
            .into_iter()
            .map(|company| CompanyOption {
                id: company.id,
                name: company.name,
            })
            .collect())
    }

    async fn tags(&self) -> Result<Vec<TagOption>, DbError> {
        let tags = self.tags.tags().await?;
        Ok(tags
            .into_iter()
            .map(|tag| TagOption {
                id: tag.id,
                name: tag.name,
                color: tag.color,
            })
            .collect())
    }

    async fn folders(&self) -> Result<Vec<FolderOption>, DbError> {
        let folders = self.folders.folders().await?;
        Ok(folders
            .into_iter()
            .map(|folder| FolderOption {
                id: folder.id,
                name: folder.name,
                parent_id: folder.parent_id,
            })
            .collect())
    }
}
