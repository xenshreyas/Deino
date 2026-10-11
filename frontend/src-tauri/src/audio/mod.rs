// src/audio/mod.rs
//
// Inherited audio code that the live recorder does not use: device listing
// and level meters for the device picker, permissions, import and
// re-transcription (with their decoder, VAD and processing helpers), and the
// legacy transcription engines. Live recording is `deino-audio` +
// `deino-recording` (FND-05b, CAP-02).
pub mod audio_processing;
pub mod decoder;
pub mod encode;
pub mod ffmpeg;
pub mod vad;

// Modularized device management
pub mod devices;
pub mod permissions;

pub mod recording_preferences;
pub mod level_monitor;
pub mod simple_level_monitor;
// GPU and performance tier detection for the Whisper engine's acceleration
pub mod hardware_detector;

// Transcription module (provider abstraction, engine management)
pub mod transcription;

// Shared utilities for import and retranscription
pub(crate) mod common;

// Shared constants
pub mod constants;

// Retranscription module (re-process stored audio with different settings)
pub mod retranscription;

// Import module (import external audio files as new meetings)
pub mod import;

pub use devices::{
    default_input_device, default_output_device, get_device_and_config, list_audio_devices,
    parse_audio_device, trigger_audio_permission,
    AudioDevice, AudioTranscriptionEngine, DeviceControl, DeviceType,
    LAST_AUDIO_CAPTURE,
};

pub use hardware_detector::{HardwareProfile, AdaptiveWhisperConfig, PerformanceTier, GpuType};
pub use recording_preferences::get_default_recordings_folder;
pub use level_monitor::{AudioLevelMonitor, AudioLevelData, AudioLevelUpdate};
pub use encode::{
    encode_single_audio, AudioInput
};

pub use vad::{extract_speech_16k};

// Export decoder for retranscription
pub use decoder::{decode_audio_file, DecodedAudio};

// Export audio constants
pub use constants::AUDIO_EXTENSIONS;
