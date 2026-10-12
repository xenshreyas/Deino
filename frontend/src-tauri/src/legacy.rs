//! Legacy command surface inherited from the upstream app (Deino's v0.4.1 base).
//!
//! Every pre-FND-02 Tauri command is registered here, in one
//! `generate_handler!`, so `lib.rs` stays a small builder. These commands are
//! app-level (not plugin) commands, so they are not ACL-gated: Deino has no
//! `AppManifest`, and adding one would put every legacy command under the ACL.
//!
//! **Do not add new commands here.** New IPC goes in an inline feature plugin
//! (PARITY_PLAN §3 rule 3). Cards port these commands to
//! feature plugins one area at a time and delete the entries they replace.
//! Until then, any change to this list goes inside a `// ── <CARD-ID> ──`
//! banner.

mod devices;

use crate::{
    api, audio, console_utils, database, notifications, parakeet_engine, summary,
    updater, whisper_engine,
};
#[cfg(target_os = "macos")]
use crate::utils;
use devices::*;

/// Returns the invoke handler for every legacy command.
///
/// This is a declarative registration list rather than logic, so it is exempt
/// from the 40-line function limit (§3A); it shrinks as cards port commands.
pub(crate) fn invoke_handler() -> impl Fn(tauri::ipc::Invoke<tauri::Wry>) -> bool + Send + Sync + 'static {
    tauri::generate_handler![
        // Devices and level meters for the device picker (legacy/devices.rs); recording is the `recording` plugin
        get_audio_devices,
        trigger_microphone_permission,
        start_audio_level_monitoring,
        stop_audio_level_monitoring,
        is_audio_level_monitoring,
        // Whisper engine
        whisper_engine::commands::whisper_init,
        whisper_engine::commands::whisper_get_available_models,
        whisper_engine::commands::whisper_load_model,
        whisper_engine::commands::whisper_get_current_model,
        whisper_engine::commands::whisper_is_model_loaded,
        whisper_engine::commands::whisper_has_available_models,
        whisper_engine::commands::whisper_validate_model_ready,
        whisper_engine::commands::whisper_transcribe_audio,
        whisper_engine::commands::whisper_get_models_directory,
        whisper_engine::commands::whisper_download_model,
        whisper_engine::commands::whisper_cancel_download,
        whisper_engine::commands::whisper_delete_corrupted_model,
        whisper_engine::commands::open_models_folder,
        // Parakeet engine
        parakeet_engine::commands::parakeet_init,
        parakeet_engine::commands::parakeet_get_available_models,
        parakeet_engine::commands::parakeet_load_model,
        parakeet_engine::commands::parakeet_get_current_model,
        parakeet_engine::commands::parakeet_is_model_loaded,
        parakeet_engine::commands::parakeet_has_available_models,
        parakeet_engine::commands::parakeet_validate_model_ready,
        parakeet_engine::commands::parakeet_transcribe_audio,
        parakeet_engine::commands::parakeet_get_models_directory,
        parakeet_engine::commands::parakeet_download_model,
        parakeet_engine::commands::parakeet_retry_download,
        parakeet_engine::commands::parakeet_cancel_download,
        parakeet_engine::commands::parakeet_delete_corrupted_model,
        parakeet_engine::commands::open_parakeet_models_folder,
        // Parallel Whisper processing
        whisper_engine::parallel_commands::initialize_parallel_processor,
        whisper_engine::parallel_commands::start_parallel_processing,
        whisper_engine::parallel_commands::pause_parallel_processing,
        whisper_engine::parallel_commands::resume_parallel_processing,
        whisper_engine::parallel_commands::stop_parallel_processing,
        whisper_engine::parallel_commands::get_parallel_processing_status,
        whisper_engine::parallel_commands::get_system_resources,
        whisper_engine::parallel_commands::check_resource_constraints,
        whisper_engine::parallel_commands::calculate_optimal_workers,
        whisper_engine::parallel_commands::prepare_audio_chunks,
        whisper_engine::parallel_commands::test_parallel_processing_setup,
        // Debug console (Windows)
        console_utils::show_console,
        console_utils::hide_console,
        console_utils::toggle_console,
        // Meetings and transcripts (list/get/rename: the meetings plugin; settings: the settings plugin)
        api::api_delete_meeting,
        api::api_get_meeting_metadata,
        api::api_get_meeting_transcripts,
        api::open_meeting_folder,
        api::open_external_url,
        api::api_test_custom_openai_connection,
        // Summaries
        // Built-in local LLM
        summary::summary_engine::commands::builtin_ai_list_models,
        summary::summary_engine::commands::builtin_ai_get_model_info,
        summary::summary_engine::commands::builtin_ai_download_model,
        summary::summary_engine::commands::builtin_ai_cancel_download,
        summary::summary_engine::commands::builtin_ai_delete_model,
        summary::summary_engine::commands::builtin_ai_is_model_ready,
        summary::summary_engine::commands::builtin_ai_get_available_summary_model,
        summary::summary_engine::commands::builtin_ai_get_recommended_model,
        // Recordings folder (preferences: the settings plugin)
        audio::recording_preferences::get_default_recordings_folder_path,
        audio::recording_preferences::open_recordings_folder,
        audio::recording_preferences::select_recording_folder,
        // Notifications
        notifications::commands::request_notification_permission,
        notifications::commands::show_notification,
        notifications::commands::show_test_notification,
        notifications::commands::is_dnd_active,
        notifications::commands::get_system_dnd_status,
        notifications::commands::set_manual_dnd,
        notifications::commands::set_notification_consent,
        notifications::commands::clear_notifications,
        notifications::commands::is_notification_system_ready,
        notifications::commands::initialize_notification_manager_manual,
        notifications::commands::test_notification_with_auto_consent,
        notifications::commands::get_notification_stats,
        // Screen and system-audio permissions
        audio::permissions::check_screen_recording_permission_command,
        audio::permissions::request_screen_recording_permission_command,
        audio::permissions::trigger_system_audio_permission_command,
        // Database lifecycle and data folder
        database::commands::check_first_launch,
        database::commands::initialize_fresh_database,
        database::commands::get_database_directory,
        database::commands::open_database_folder,
        // macOS System Settings deep links
        #[cfg(target_os = "macos")]
        utils::open_system_settings,
        // Retranscription and audio import
        audio::retranscription::start_retranscription_command,
        audio::retranscription::cancel_retranscription_command,
        audio::retranscription::is_retranscription_in_progress_command,
        audio::import::select_and_validate_audio_command,
        audio::import::validate_audio_file_command,
        audio::import::start_import_audio_command,
        audio::import::cancel_import_command,
        audio::import::is_import_in_progress_command,
        // ── FND-01 updater ──
        updater::updates_enabled,
    ]
}
