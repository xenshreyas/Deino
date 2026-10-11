// audio/transcription/mod.rs
//
// Transcription module: provider abstraction and engine management (the
// legacy engines; the live recorder's utterance lanes and import use them).

pub mod provider;
pub mod whisper_provider;
pub mod parakeet_provider;
pub mod engine;

// Re-export commonly used types
pub use provider::{TranscriptionError, TranscriptionProvider, TranscriptResult};
pub use whisper_provider::WhisperProvider;
pub use parakeet_provider::ParakeetProvider;
pub use engine::{
    TranscriptionEngine,
    validate_transcription_model_ready,
    get_or_init_transcription_engine,
    get_or_init_whisper,
};
