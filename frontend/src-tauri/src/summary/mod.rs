//! Legacy template summaries: IPC, background jobs and templates.
//!
//! Generation goes through the LLM gateway (FND-09):
//! `deino_core::summaries::SummaryGenerator` over `deino_llm::Gateway`.
//! This module keeps the legacy commands, the `summary_processes`
//! lifecycle ([`service`]), the stored result shape ([`result`]), the JSON
//! templates and the built-in model manager (`summary_engine`).

pub mod commands;
mod result;
pub mod service;
pub mod summary_engine;
pub mod template_commands;
pub mod templates;

// Re-export Tauri commands (with their generated __cmd__ variants)
pub use commands::{
    __cmd__api_cancel_summary, __cmd__api_get_summary, __cmd__api_process_transcript,
    __cmd__api_save_meeting_summary, __tauri_command_name_api_cancel_summary,
    __tauri_command_name_api_get_summary, __tauri_command_name_api_process_transcript,
    __tauri_command_name_api_save_meeting_summary, api_cancel_summary, api_get_summary,
    api_process_transcript, api_save_meeting_summary,
};

// Re-export template commands
pub use template_commands::{
    __cmd__api_get_template_details, __cmd__api_list_templates, __cmd__api_validate_template,
    __tauri_command_name_api_get_template_details, __tauri_command_name_api_list_templates,
    __tauri_command_name_api_validate_template,
    api_get_template_details, api_list_templates, api_validate_template,
};

pub use service::SummaryService;
