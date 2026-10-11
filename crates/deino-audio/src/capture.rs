//! Capture: the real audio sources and the machinery that keeps them alive.
//!
//! ```text
//!  mic (cpal, own thread) ─RT─▶ capture ring ─▶ conditioner ─48 kHz─▶ RtSink ─▶ mic writer
//!  tap (Core Audio IOProc) ─RT─▶ capture ring ─▶ conditioner ─48 kHz─▶ RtSink ─▶ system writer
//!                                                 ▲
//!  device listeners, sleep/wake, 1 s tick ─▶ supervisor (restart, backoff, pause, route)
//! ```
//!
//! | Module | Role |
//! |---|---|
//! | [`sources`] | `DeviceSources`: the real devices (`SourceFactory`) |
//! | [`mic`] | `MicSource`: cpal input on a dedicated thread, device ranking, `alive` flag |
//! | [`macos`] | The process tap, Core Audio listeners, sleep/wake, the route probe, the permission probe (macOS only) |
//! | [`conditioner`] | Native rate → 48 kHz, onto the pause-free clock |
//! | [`resample`] | `ChunkResampler`: one allocation-free rubato conversion |
//! | [`track`] | `TrackCapture`: a source and its conditioner thread |
//! | [`supervisor`] | `Capture`: the per-recording supervisor thread |
//! | [`watchdog`] | `TrackWatch`: stall detection and restart backoff (pure) |
//! | [`devices`] | Which microphone to open (pure ranking, HFP rule) |
//! | [`route`] | Speakers or headphones (pure verdict, two-probe debounce) |
//! | [`events`] | Hardware and power events, and the `SystemWatch` port |
//! | [`meter`] | Per-track activity atomics and the pause ledger |
//! | [`state`] | Each track's meter and status for the whole recording (`TrackStates`) |
//!
//! Real-time callbacks only push into a preallocated ring and touch atomics
//! (§3C); everything else runs on ordinary threads.

pub mod conditioner;
pub mod devices;
pub mod events;
#[cfg(target_os = "macos")]
pub mod macos;
pub mod meter;
pub mod mic;
pub mod resample;
pub mod route;
pub mod sources;
pub mod state;
pub mod supervisor;
pub mod track;
pub mod watchdog;

pub use conditioner::CaptureShared;
pub use devices::{InputDevice, InputDevices, MicChoice, Transport};
pub use events::{DeviceEvent, NoSystemWatch, PowerEvent, SystemEvent, SystemWatch};
pub use mic::MicSource;
pub use route::{FixedRoute, OutputRoute, RouteProbe};
pub use sources::{DeviceSources, Unavailable};
pub use state::{TrackState, TrackStates, TrackStatus};
pub use supervisor::{Capture, CaptureInputs, CaptureParts};
pub use track::TrackCapture;
