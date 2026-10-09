//! [`FolderService`]: the folder tree's rules.
//!
//! Names are unique among siblings (ignoring case), moves never make a
//! cycle, and deleting a folder deletes its subfolders while every meeting
//! stays where else it was filed. Filing meetings is [`FilingService`](super::FilingService).

use std::collections::HashMap;
use std::sync::Arc;

use deino_domain::{Clock, FolderId};

use super::{
    Folder, FolderDescription, FolderError, FolderIcon, FolderMove, FolderName, FolderPorts,
    FolderRepo, FolderTree, MeetingFolderRepo, tree,
};

/// A folder to create.
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct NewFolder {
    /// Its name; trimmed, unique among its siblings.
    pub name: String,
    /// The folder to create it in; `None` for the top level.
    pub parent_id: Option<FolderId>,
}

/// Creates, edits, moves and deletes folders.
pub struct FolderService {
    folders: Arc<dyn FolderRepo>,
    filing: Arc<dyn MeetingFolderRepo>,
    clock: Arc<dyn Clock>,
}

impl FolderService {
    /// Creates the service over its ports.
    pub fn new(ports: &FolderPorts) -> Self {
        Self {
            folders: Arc::clone(&ports.folders),
            filing: Arc::clone(&ports.filing),
            clock: Arc::clone(&ports.clock),
        }
    }

    /// Every folder in display order, with meeting counts and the size of
    /// the "Unfiled" smart folder.
    pub async fn tree(&self) -> Result<FolderTree, FolderError> {
        let folders = self.folders.folders().await?;
        let counts: HashMap<FolderId, u32> =
            self.filing.meeting_counts().await?.into_iter().collect();
        Ok(FolderTree {
            entries: tree::display_order(folders, &counts),
            unfiled_count: self.filing.unfiled_count().await?,
        })
    }

    /// The folder with `id`.
    pub async fn get(&self, id: &FolderId) -> Result<Folder, FolderError> {
        self.folders
            .folder(id)
            .await?
            .ok_or(FolderError::NotFound(*id))
    }

    /// Creates a folder at the top level or inside `parent_id`.
    pub async fn create(&self, new: &NewFolder) -> Result<Folder, FolderError> {
        let name = FolderName::parse(&new.name)?;
        if let Some(parent) = &new.parent_id {
            self.get(parent).await?;
        }
        self.ensure_name_free(new.parent_id.as_ref(), name.as_str(), None)
            .await?;
        let now = self.clock.now();
        let folder = Folder {
            id: FolderId::generate(self.clock.as_ref()),
            parent_id: new.parent_id,
            name: name.as_str().to_owned(),
            icon: None,
            description: String::new(),
            created_at: now,
            updated_at: now,
        };
        self.folders
            .create_folder(&folder)
            .await
            .map_err(FolderError::naming)?;
        log::info!(target: "folders", "created folder {}", folder.id);
        Ok(folder)
    }

    /// Renames the folder.
    pub async fn rename(&self, id: &FolderId, name: &str) -> Result<Folder, FolderError> {
        let name = FolderName::parse(name)?;
        let folder = self.get(id).await?;
        self.ensure_name_free(folder.parent_id.as_ref(), name.as_str(), Some(id))
            .await?;
        self.save(Folder {
            name: name.as_str().to_owned(),
            ..folder
        })
        .await
    }

    /// Sets the folder's icon; `None` restores the default.
    pub async fn set_icon(&self, id: &FolderId, icon: Option<&str>) -> Result<Folder, FolderError> {
        let icon = icon.map(FolderIcon::parse).transpose()?;
        let folder = self.get(id).await?;
        self.save(Folder {
            icon: icon.map(|icon| icon.as_str().to_owned()),
            ..folder
        })
        .await
    }

    /// Sets what the folder is for (context for chat and auto-filing).
    pub async fn set_description(&self, id: &FolderId, text: &str) -> Result<Folder, FolderError> {
        let description = FolderDescription::parse(text)?;
        let folder = self.get(id).await?;
        self.save(Folder {
            description: description.as_str().to_owned(),
            ..folder
        })
        .await
    }

    /// Moves the folder, with its subfolders and every meeting filed in
    /// them, under `parent` (`None`: the top level).
    pub async fn move_to(
        &self,
        id: &FolderId,
        parent: Option<&FolderId>,
    ) -> Result<Folder, FolderError> {
        let folder = self.get(id).await?;
        if let Some(parent) = parent {
            self.get(parent).await?;
        }
        self.ensure_name_free(parent, &folder.name, Some(id))
            .await?;
        let moved = self
            .folders
            .move_folder(id, parent, self.clock.now())
            .await
            .map_err(FolderError::naming)?;
        match moved {
            FolderMove::Moved => self.get(id).await,
            FolderMove::NotFound => Err(FolderError::NotFound(*id)),
            FolderMove::WouldCycle => Err(FolderError::WouldCycle),
        }
    }

    /// Deletes the folder and its subfolders. Their meetings are kept; they
    /// only leave these folders.
    pub async fn delete(&self, id: &FolderId) -> Result<(), FolderError> {
        if !self.folders.delete_folder(id).await? {
            return Err(FolderError::NotFound(*id));
        }
        log::info!(target: "folders", "deleted folder {id} and its subfolders");
        Ok(())
    }

    /// Fails with [`FolderError::NameTaken`] when a sibling under `parent`,
    /// other than `except`, already has `name` (ignoring case).
    async fn ensure_name_free(
        &self,
        parent: Option<&FolderId>,
        name: &str,
        except: Option<&FolderId>,
    ) -> Result<(), FolderError> {
        let siblings = self.folders.children(parent).await?;
        let wanted = name.to_lowercase();
        let taken = siblings
            .iter()
            .any(|sibling| Some(&sibling.id) != except && sibling.name.to_lowercase() == wanted);
        if taken {
            Err(FolderError::NameTaken)
        } else {
            Ok(())
        }
    }

    /// Stores the folder's editable fields, stamped now, and returns it.
    async fn save(&self, folder: Folder) -> Result<Folder, FolderError> {
        let folder = Folder {
            updated_at: self.clock.now(),
            ..folder
        };
        if !self
            .folders
            .update_folder(&folder)
            .await
            .map_err(FolderError::naming)?
        {
            return Err(FolderError::NotFound(folder.id));
        }
        Ok(folder)
    }
}
