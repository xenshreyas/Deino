//! The real engine: transcribe-cpp 0.3.1 (ggml, Metal on Apple Silicon).
//!
//! Runs only inside a worker process. Loads GGUF models (Parakeet Unified,
//! Whisper GGUFs) and legacy whisper.cpp `.bin` files, warms each model with
//! one second of silence so the user's first words do not pay for kernel
//! compilation, and turns transcribe-cpp's snapshots into token deltas: a
//! stream reports only the tokens committed since its last answer.

mod options;
mod phrases;
mod stream;

use std::path::Path;

use transcribe_cpp::{
    Backend, BackendMask, Error, Model, ModelOptions, RunOptions, Session, Transcript,
};

use options::{loaded_info, run_options, stream_options};
use phrases::tokens_or_phrases;
pub use stream::CppStream;
use stream::timed_token;

use super::Engine;
use crate::pcm::SAMPLE_RATE_HZ;
use crate::protocol::{
    BackendChoice, BatchParams, BatchTranscript, LoadRequest, LoadedInfo, RemoteError,
    RemoteErrorKind, StreamParams,
};
use crate::window::StreamWindow;

/// Seconds of silence run through a stream after a load, to compile the GPU
/// kernels before the user's first words (STT spike, §9: 2 s, then finalize).
const WARM_UP_SECONDS: usize = 2;

/// transcribe-cpp behind the [`Engine`] contract.
#[derive(Default)]
pub struct TranscribeCppEngine {
    loaded: Option<Loaded>,
}

/// A loaded model's session and the options every call uses.
struct Loaded {
    session: Session,
    options: RunOptions,
}

impl Engine for TranscribeCppEngine {
    type Stream<'a> = CppStream<'a>;

    fn init(&mut self, cpu_only: bool) -> Result<(), RemoteError> {
        transcribe_cpp::init_logging();
        let registered = if cpu_only {
            transcribe_cpp::init_backends_with(None::<&Path>, BackendMask::CPU)
        } else {
            transcribe_cpp::init_backends_default()
        };
        registered.map_err(|error| remote(RemoteErrorKind::BackendInit, &error))
    }

    fn warm_gpu(&mut self) {
        let devices = transcribe_cpp::devices();
        log::debug!("transcribe-cpp opened {} compute devices", devices.len());
    }

    fn load(&mut self, request: &LoadRequest) -> Result<LoadedInfo, RemoteError> {
        // Free the old model first so two are never held at once.
        self.loaded = None;
        let backend = match request.backend {
            BackendChoice::Auto => Backend::Auto,
            BackendChoice::Cpu => Backend::Cpu,
        };
        let options = ModelOptions {
            backend,
            device: None,
        };
        let load_error = |error: Error| remote(RemoteErrorKind::Load, &error);
        let model = Model::load_with(&request.path, &options).map_err(load_error)?;
        let session = model.session().map_err(load_error)?;
        let capabilities = model.capabilities();
        let info = loaded_info(&model, &capabilities);
        let mut loaded = Loaded {
            session,
            options: run_options(&capabilities),
        };
        loaded.warm_up(capabilities.supports_streaming);
        self.loaded = Some(loaded);
        Ok(info)
    }

    fn run(&mut self, pcm: &[f32], _params: &BatchParams) -> Result<BatchTranscript, RemoteError> {
        let loaded = self.loaded.as_mut().ok_or_else(not_loaded)?;
        match loaded.session.run(pcm, &loaded.options) {
            Ok(transcript) => Ok(batch_transcript(&transcript)),
            Err(error) => partial_or_error(error),
        }
    }

    fn open_stream(&mut self, params: &StreamParams) -> Result<CppStream<'_>, RemoteError> {
        let loaded = self.loaded.as_mut().ok_or_else(not_loaded)?;
        let options = loaded.options.clone();
        let window = params
            .window
            .filter(|window| accepts_window(&mut loaded.session, &options, *window));
        let stream = loaded
            .session
            .stream(&options, &stream_options(window))
            .map_err(|error| remote(RemoteErrorKind::Unsupported, &error))?;
        Ok(CppStream { stream, sent: 0 })
    }
}

/// Whether the model streams with `window`: it must be in the model's
/// training menu. Opens and abandons a probe stream; a refused window falls
/// back to the model's default instead of failing the recording.
fn accepts_window(session: &mut Session, options: &RunOptions, window: StreamWindow) -> bool {
    match session.stream(options, &stream_options(Some(window))) {
        Ok(_probe) => true,
        Err(error) => {
            log::warn!("stream window {window:?} refused ({error}); using the model default");
            false
        }
    }
}

impl Loaded {
    /// Runs silence through the live path (a stream with the live window,
    /// or a one-shot run for models that cannot stream). Results and errors
    /// do not matter; only the compiled kernels do.
    fn warm_up(&mut self, streaming: bool) {
        let silence = vec![0.0_f32; WARM_UP_SECONDS * SAMPLE_RATE_HZ as usize];
        let warmed = if streaming {
            let options = stream_options(Some(StreamWindow::LIVE));
            self.session
                .stream(&self.options, &options)
                .and_then(|mut stream| {
                    stream.feed(&silence)?;
                    stream.finalize().map(drop)
                })
        } else {
            self.session.run(&silence, &self.options).map(drop)
        };
        if let Err(error) = warmed {
            log::debug!("warm-up failed: {error}");
        }
    }
}

fn batch_transcript(transcript: &Transcript) -> BatchTranscript {
    BatchTranscript {
        tokens: tokens_or_phrases(transcript)
            .unwrap_or_else(|| transcript.tokens.iter().map(timed_token).collect()),
        text: transcript.text.trim().to_string(),
    }
}

/// Keeps the partial transcript of a run stopped by a repetition or length
/// guard: an incomplete transcript beats none (audio-stt.md, CAP-08 notes).
fn partial_or_error(error: Error) -> Result<BatchTranscript, RemoteError> {
    match error {
        Error::Aborted {
            partial: Some(partial),
            ..
        }
        | Error::OutputTruncated {
            partial: Some(partial),
            ..
        }
        | Error::OutputRepetition {
            partial: Some(partial),
            ..
        } => {
            log::warn!("transcription stopped early; keeping the partial transcript");
            Ok(batch_transcript(&partial))
        }
        other => Err(remote(RemoteErrorKind::Inference, &other)),
    }
}

fn not_loaded() -> RemoteError {
    RemoteError::new(RemoteErrorKind::NotLoaded, "load a model first")
}

pub(super) fn remote(kind: RemoteErrorKind, error: &Error) -> RemoteError {
    RemoteError::new(kind, error.to_string())
}
