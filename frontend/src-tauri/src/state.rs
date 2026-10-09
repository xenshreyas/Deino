//! Tauri-managed application state.
//!
//! [`AppState`] is registered once the database is open (see
//! `database::setup`); everything else is registered on the builder by
//! [`manage_initial_state`] before the app starts.

use std::sync::Arc;

use tauri::{Builder, Wry};
use tokio::sync::{Mutex, RwLock};

use crate::database::manager::DatabaseManager;
use crate::notifications::{commands::NotificationManagerState, manager::NotificationManager};
use crate::{audio, summary, whisper_engine};

/// State available after the database has been opened.
pub struct AppState {
    pub db_manager: DatabaseManager,
}

/// Registers the state that legacy commands expect from the first frame.
pub(crate) fn manage_initial_state(builder: Builder<Wry>) -> Builder<Wry> {
    let notifications: NotificationManagerState<Wry> =
        Arc::new(RwLock::new(None::<NotificationManager<Wry>>));
    builder
        .manage(whisper_engine::parallel_commands::ParallelProcessorState::new())
        .manage(notifications)
        .manage(audio::init_system_audio_state())
        .manage(summary::summary_engine::ModelManagerState(Arc::new(Mutex::new(None))))
}
