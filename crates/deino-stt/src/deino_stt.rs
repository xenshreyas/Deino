//! Deino's speech-to-text engine: Parakeet Unified EN 0.6B (and Whisper)
//! through transcribe-cpp, isolated in worker processes.
//!
//! ```text
//!  app process                                   worker process (same executable)
//!  ┌──────────────────────────────────────┐      ┌─────────────────────────────┐
//!  │ LiveSession (one thread per lane)    │      │ worker::run                 │
//!  │   retained audio · Assembler · Perf  │      │   serve(Engine)             │
//!  │        │ feed / finalize (FIFO)      │      │   TranscribeCppEngine       │
//!  │ EngineSupervisor (owner thread) ─────┼─────►│   (Model, Session, Stream)  │
//!  │   deadlines · GPU→CPU · kill=cancel  │◄─────┤ frames on stdin / stdout    │
//!  └──────────────────────────────────────┘      └─────────────────────────────┘
//! ```
//!
//! - [`supervisor`]: [`EngineSupervisor`] owns one worker, absorbs crashes and
//!   hangs, and falls back from the GPU to the CPU.
//! - [`worker`]: the worker process's request loop; [`engine`] the model.
//! - [`session`]: a live lane. It feeds continuous (never VAD-gated) audio so
//!   token times stay on the recording's timeline, turns committed tokens into
//!   segments, and re-transcribes its retained audio when a stream is lost.
//! - [`pool`]: one supervisor per lane plus one for batch work, so the
//!   microphone and system lanes stream concurrently (one stream per model).
//! - [`perf`]: `StreamPerf`, the real-time factor, backlog and first-text
//!   latency, logged without text. [`bench`]: the headless benchmark.
//!
//! This crate never touches the network (PARITY_PLAN §4.0): models are
//! downloaded by `deino-core::models` and handed in as paths.

pub mod bench;
pub mod clock;
pub mod engine;
mod error;
pub mod pcm;
pub mod perf;
pub mod pool;
pub mod protocol;
pub mod session;
pub mod supervisor;
pub mod transcriber;
pub mod window;
pub mod words;
pub mod worker;

pub use error::{SttError, WorkerFault};
pub use pcm::{Millis, SAMPLE_RATE_HZ};
pub use pool::{EnginePool, EngineSlot, Lane, PinGuard};
pub use protocol::{BackendChoice, BatchTranscript, LoadedInfo, StreamParams};
pub use session::{
    FinalSegment, LaneStatus, LiveSession, LiveText, SegmentOrigin, SegmentPolicy, SessionConfig,
    SessionSink, StatusHandle,
};
pub use supervisor::{EngineSupervisor, LoadSpec, StreamHandle, SupervisorConfig, WorkerLauncher};
pub use transcriber::{BatchTranscriber, StreamingTranscriber, Transcriber, TranscriptStream};
pub use window::StreamWindow;
pub use words::{TimedToken, TimedWord};
