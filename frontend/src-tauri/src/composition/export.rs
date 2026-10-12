//! Wiring for exports (EXP-01, EXP-02).
//!
//! One [`ExportAssembler`] over the SQLite repositories and the Mac's time
//! zone feeds three services: [`ExportService`] (preview, save, copy,
//! email), [`AutoExportService`] (the `auto_export` stage's port and the
//! folder picker) and [`FullExportService`] (the account export). The OS
//! adapters are the export plugin's (`features/export/platform.rs`); the
//! dialogs are also managed as ports for the folder picker and the account
//! export's save dialog. Exports are redacted by patterns alone when the
//! user asks (NOT-09, ADR 0038): no model call on preview, copy or save.

use std::sync::Arc;

use deino_core::export::{
    AutoExportService, AutoExporter, ExportAssembler, ExportPorts, ExportService,
    ExportServiceDeps, FolderDialog, FsVault, FullExportDeps, FullExportService, SaveDialog,
    SystemZone, Vault,
};
use deino_core::meetings::SqliteMeetingRepo;
use deino_core::redaction::{Redactor, SettingRedactor};
use deino_core::settings::SettingsService;
use deino_db::Database;
use deino_domain::Clock;
use tauri::{AppHandle, Manager, Runtime};

use crate::features::export::{SystemClipboard, SystemOpener, TauriDialogs, TauriFullExportEvents};

/// The export services, built once the database is open.
pub(crate) struct ExportServices {
    export: Arc<ExportService>,
    auto: Arc<AutoExportService>,
    full: Arc<FullExportService>,
    dialogs: Arc<TauriDialogs<tauri::Wry>>,
}

impl ExportServices {
    /// Builds every export service over `database`.
    pub(crate) fn build(
        app: &AppHandle,
        database: &Database,
        settings: Arc<SettingsService>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        let redactor = SettingRedactor::new(settings.clone(), Arc::new(Redactor::regex_only()));
        let ports =
            ExportPorts::sqlite(database, Arc::new(SystemZone)).with_redactor(Arc::new(redactor));
        let assembler = Arc::new(ExportAssembler::new(ports));
        let vault: Arc<dyn Vault> = Arc::new(FsVault);
        let dialogs = Arc::new(TauriDialogs::new(app.clone()));
        let export = ExportService::new(ExportServiceDeps {
            assembler: Arc::clone(&assembler),
            vault: Arc::clone(&vault),
            save_dialog: dialogs.clone(),
            clipboard: Arc::new(SystemClipboard),
            opener: Arc::new(SystemOpener),
        });
        let full = FullExportService::new(FullExportDeps {
            assembler: Arc::clone(&assembler),
            lifecycle: Arc::new(SqliteMeetingRepo::new(database.clone())),
            events: Arc::new(TauriFullExportEvents::new(app.clone())),
            clock,
        });
        Self {
            export: Arc::new(export),
            auto: Arc::new(AutoExportService::new(assembler, settings, vault)),
            full: Arc::new(full),
            dialogs,
        }
    }

    /// The `auto_export` stage's port.
    pub(crate) fn auto_exporter(&self) -> Arc<dyn AutoExporter> {
        self.auto.clone()
    }

    /// Registers the services and the dialog ports as Tauri state.
    pub(crate) fn manage<R: Runtime>(self, app: &impl Manager<R>) {
        let folder_dialog: Arc<dyn FolderDialog> = self.dialogs.clone();
        let save_dialog: Arc<dyn SaveDialog> = self.dialogs;
        app.manage(self.export);
        app.manage(self.auto);
        app.manage(self.full);
        app.manage(folder_dialog);
        app.manage(save_dialog);
    }
}
