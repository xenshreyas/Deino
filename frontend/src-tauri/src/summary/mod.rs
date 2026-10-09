//! Legacy template summaries: IPC, background jobs and templates.
//!
//! Generation goes through the LLM gateway (FND-09):
//! `deino_core::summaries::SummaryGenerator` over `deino_llm::Gateway`.
//! This module keeps the legacy commands, the `summary_processes`
//! lifecycle ([`service`]), the stored result shape ([`result`]) and the
//! built-in model manager (`summary_engine`). Templates come from the
//! `templates` table (NOT-04) through `features::templates::summary_template`.

pub mod commands;
mod result;
pub mod service;
pub mod summary_engine;

// Re-export Tauri commands (with their generated __cmd__ variants)
pub use commands::{
    __cmd__api_cancel_summary, __cmd__api_get_summary, __cmd__api_process_transcript,
    __cmd__api_save_meeting_summary, __tauri_command_name_api_cancel_summary,
    __tauri_command_name_api_get_summary, __tauri_command_name_api_process_transcript,
    __tauri_command_name_api_save_meeting_summary, api_cancel_summary, api_get_summary,
    api_process_transcript, api_save_meeting_summary,
};

pub use service::SummaryService;
