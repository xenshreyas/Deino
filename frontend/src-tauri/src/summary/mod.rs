//! Summary module - handles all meeting summary generation functionality
//!
//! This module contains:
//! - LLM client for communicating with various AI providers (OpenAI, Claude, Groq, Ollama, OpenRouter, CustomOpenAI)
//! - Processor for chunking transcripts and generating summaries
//! - Service layer for orchestrating summary generation
//! - Templates for structured meeting summary generation
//! - Tauri commands for frontend integration

pub mod commands;
pub(crate) mod credentials;
pub mod llm_client;
pub mod processor;
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

// Re-export commonly used items
pub use llm_client::LLMProvider;
pub use processor::{chunk_text, extract_meeting_name_from_markdown, rough_token_count};
pub use service::SummaryService;
