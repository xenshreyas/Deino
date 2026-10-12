//! Wiring for onboarding's permissions step (FND-11): the permission rules
//! over this app's operating-system permissions.

use std::sync::Arc;

use deino_core::permissions::PermissionService;
use tauri::{AppHandle, Manager, Runtime};

use crate::permissions::SystemPermissions;

/// Manages `Arc<PermissionService>` over the real permissions.
pub(crate) fn attach_permissions<R: Runtime>(app: &AppHandle<R>) {
    let gate = Arc::new(SystemPermissions::new(app.clone()));
    app.manage(Arc::new(PermissionService::new(gate)));
}
