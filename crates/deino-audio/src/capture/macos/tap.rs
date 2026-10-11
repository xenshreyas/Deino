//! `SystemTap`: everything the Mac plays, through a Core Audio process tap.
//!
//! The tap is a mono global tap that **excludes Deino's own process**, so
//! the app's own sounds never reach the "Them" track (muesli). It feeds a
//! private, tap-only aggregate device with `tap_auto_start = false`, so the
//! device's IO cycle runs (and the watchdog sees frames) even in silence.
//!
//! The aggregate has a **stable UID**, [`TAP_UID`], with one deterministic
//! fallback, [`TAP_FALLBACK_UID`]. Private aggregates are invisible to
//! device enumeration, so a crashed session's device cannot be swept; a
//! fresh UUID per run would leak HAL settings entries instead (muesli's
//! reasoning). If both UIDs are held by leftovers, starting fails and the
//! recording continues with the microphone only.
//!
//! Teardown is RAII in field order: the started device stops, then the IO
//! context and the liveness listener go, then the tap is destroyed.

mod io_proc;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::channel;
use std::time::Duration;

use cidre::{arc, av, cf, core_audio as ca, ns, os};

use self::io_proc::{TapContext, io_proc};
use super::listener::Listener;
use crate::ring::capture_ring;
use crate::source::{AudioSource, SourceError, SourceStream};
use crate::units::{SampleRate, Track};

/// The aggregate device's stable UID.
pub const TAP_UID: &str = "com.xenshreyas.deino.system-audio-tap";

/// The UID used when a leftover holds [`TAP_UID`].
pub const TAP_FALLBACK_UID: &str = "com.xenshreyas.deino.system-audio-tap.fallback";

/// The aggregate device's name.
pub const TAP_NAME: &str = "Deino System Audio";

/// How long a new aggregate device may take to come alive (openwhispr).
const READY_TIMEOUT: Duration = Duration::from_secs(1);

/// How often readiness is checked.
const READY_POLL: Duration = Duration::from_millis(20);

/// A running tap. Fields drop in order: device, liveness listener, context, tap.
struct RunningTap {
    _device: ca::hardware::StartedDevice<ca::AggregateDevice>,
    _alive_watch: Option<Listener<Arc<AtomicBool>>>,
    _context: Box<TapContext>,
    _tap: ca::TapGuard,
}

impl std::fmt::Debug for RunningTap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RunningTap").finish_non_exhaustive()
    }
}

// SAFETY: the Core Audio objects are plain IDs usable from any thread, the
// context is only read by the IOProc, and the listener context is atomic.
#[allow(
    unsafe_code,
    reason = "Core Audio object IDs and the IOProc context cross threads"
)]
unsafe impl Send for RunningTap {}

/// The system-audio source.
#[derive(Debug, Default)]
pub struct SystemTap {
    running: Option<RunningTap>,
    alive: Arc<AtomicBool>,
}

impl SystemTap {
    /// A tap that is not started yet.
    pub fn new() -> Self {
        Self::default()
    }

    fn open(&mut self) -> Result<SourceStream, SourceError> {
        let tap = create_tap().map_err(os_error)?;
        let asbd = tap.asbd().map_err(os_error)?;
        let format = av::AudioFormat::with_asbd(&asbd)
            .map_or(av::audio::CommonFormat::Other, |f| f.common_format());
        let rate = SampleRate(rate_of(asbd.sample_rate));
        let (sink, blocks) = capture_ring(rate.0);
        let timebase = cidre::mach::TimeBaseInfo::new();
        let mut context = Box::new(TapContext {
            sink,
            format,
            channels: usize::try_from(asbd.channels_per_frame).unwrap_or(1),
            timebase: (u64::from(timebase.numer), u64::from(timebase.denom)),
        });
        let aggregate = create_aggregate(&tap).map_err(os_error)?;
        wait_until_alive(&aggregate);
        self.alive.store(true, Ordering::Release);
        let alive_watch = watch_alive(&aggregate, &self.alive);
        let proc_id = aggregate
            .create_io_proc_id(io_proc, Some(&mut *context))
            .map_err(os_error)?;
        let device = ca::device_start(aggregate, Some(proc_id)).map_err(os_error)?;
        self.running = Some(RunningTap {
            _device: device,
            _alive_watch: alive_watch,
            _context: context,
            _tap: tap,
        });
        log::info!(target: "deino_audio::capture", "system audio tap at {rate}");
        Ok(SourceStream { rate, blocks })
    }
}

impl AudioSource for SystemTap {
    fn track(&self) -> Track {
        Track::System
    }

    fn start(&mut self) -> Result<SourceStream, SourceError> {
        if self.running.is_some() {
            return Err(SourceError::AlreadyStarted(Track::System));
        }
        self.open()
    }

    fn stop(&mut self) {
        self.running = None;
    }

    fn is_alive(&self) -> bool {
        self.running.is_some() && self.alive.load(Ordering::Acquire)
    }
}

/// A mono global tap of every process but this one.
fn create_tap() -> os::Result<ca::TapGuard> {
    let excluded = own_process().map(|process| ns::Number::with_u32(process.0.0));
    let numbers: Vec<&ns::Number> = excluded.iter().map(|number| &**number).collect();
    let mut desc =
        ca::TapDesc::with_mono_global_tap_excluding_processes(&ns::Array::from_slice(&numbers));
    desc.set_private(true);
    desc.set_mute_behavior(ca::TapMuteBehavior::Unmuted);
    desc.create_process_tap()
}

/// This process's Core Audio object, when it has one.
fn own_process() -> Option<ca::Process> {
    let pid = i32::try_from(std::process::id()).ok()?;
    let process = ca::Process::with_pid(pid).ok()?;
    (process.0.0 != 0).then_some(process)
}

/// The private, tap-only aggregate device, on the stable UID or its fallback.
fn create_aggregate(tap: &ca::TapGuard) -> os::Result<ca::AggregateDevice> {
    let tap_uid = tap.uid()?;
    match ca::AggregateDevice::with_desc(&aggregate_desc(&tap_uid, TAP_UID)) {
        Ok(device) => Ok(device),
        Err(status) => {
            log::warn!(target: "deino_audio::capture", "tap UID taken (OSStatus {status:?}); using the fallback");
            ca::AggregateDevice::with_desc(&aggregate_desc(&tap_uid, TAP_FALLBACK_UID))
        }
    }
}

fn aggregate_desc(
    tap_uid: &cf::String,
    uid: &str,
) -> arc::R<cf::DictionaryOf<cf::String, cf::Type>> {
    use ca::aggregate_device_keys as keys;
    use ca::hardware::sub_tap_keys;
    let sub_tap = cf::DictionaryOf::with_keys_values(
        &[sub_tap_keys::uid(), sub_tap_keys::drift_compensation()],
        &[
            tap_uid.as_type_ref(),
            cf::Boolean::value_true().as_type_ref(),
        ],
    );
    let name = cf::String::from_str(TAP_NAME);
    let uid = cf::String::from_str(uid);
    let taps = cf::ArrayOf::from_slice(&[sub_tap.as_ref()]);
    cf::DictionaryOf::with_keys_values(
        &[
            keys::is_private(),
            keys::tap_auto_start(),
            keys::name(),
            keys::uid(),
            keys::tap_list(),
        ],
        &[
            cf::Boolean::value_true().as_type_ref(),
            cf::Boolean::value_false().as_type_ref(),
            name.as_type_ref(),
            uid.as_type_ref(),
            taps.as_type_ref(),
        ],
    )
}

/// Waits up to [`READY_TIMEOUT`] for the new device to report itself alive.
fn wait_until_alive(device: &ca::AggregateDevice) {
    let (_keep, timer) = channel::<()>();
    let alive = ca::PropSelector::DEVICE_IS_ALIVE.global_addr();
    for _ in 0..READY_TIMEOUT.as_millis() / READY_POLL.as_millis() {
        if device.bool_prop(&alive).unwrap_or(true) {
            return;
        }
        timer.recv_timeout(READY_POLL).ok();
    }
}

/// Clears `alive` when the aggregate device dies (tap invalidated, coreaudiod restart).
fn watch_alive(
    device: &ca::AggregateDevice,
    alive: &Arc<AtomicBool>,
) -> Option<Listener<Arc<AtomicBool>>> {
    let object: &ca::Device = device;
    let address = vec![ca::PropSelector::DEVICE_IS_ALIVE.global_addr()];
    Listener::register(object.0, address, Arc::clone(alive), on_alive_change).ok()
}

#[allow(
    unsafe_code,
    reason = "a Core Audio listener callback reading its context"
)]
extern "C-unwind" fn on_alive_change(
    object: ca::Obj,
    _count: u32,
    _addresses: *const ca::PropAddr,
    context: *mut Arc<AtomicBool>,
) -> os::Status {
    if context.is_null() {
        return os::Status::NO_ERR;
    }
    let alive = object
        .bool_prop(&ca::PropSelector::DEVICE_IS_ALIVE.global_addr())
        .unwrap_or(false);
    // SAFETY: the context registered with this callback lives until the
    // listener is removed.
    unsafe { &*context }.store(alive, Ordering::Release);
    os::Status::NO_ERR
}

/// A device rate as whole frames per second.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "device rates are positive whole numbers far below u32::MAX"
)]
fn rate_of(rate: f64) -> u32 {
    rate.round().max(1.0) as u32
}

fn os_error(error: os::Error) -> SourceError {
    SourceError::Open {
        track: Track::System,
        source: Box::new(OsStatus(error.0.get())),
    }
}

/// A failing Core Audio call's status.
#[derive(Debug, thiserror::Error)]
#[error("Core Audio returned OSStatus {0}")]
struct OsStatus(i32);
