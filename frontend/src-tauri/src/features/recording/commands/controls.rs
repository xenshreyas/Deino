//! The record panel's and auto-stop's commands (CAP-10, CAP-07): switch
//! the microphone, turn system audio off and on, list microphones, keep
//! recording, answer the "new note" offer, open the live meeting, and the
//! pill's rectangle for the panel's click-through.

use std::sync::Arc;

use deino_audio::capture::devices::{InputDevices, Transport, is_loopback};
use deino_audio::capture::mic::CpalInputs;
use deino_core::settings::PanelEdge;
use deino_detect::autostop::{AutoStopService, AutoStopState};
use deino_domain::{AppError, ErrorCode, MeetingId};
use deino_recording::{RecordingService, RecordingSnapshot};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Runtime, State};
use tauri_specta::Event;

use crate::error::IntoIpc;
use crate::features::recording::events::OpenLiveMeeting;
use crate::features::recording::panel::{self, PanelHost, placement::Rect};

/// A microphone the panel's picker offers.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Microphone {
    /// Its name, which the recorder is switched to.
    pub name: String,
    /// The system's default input.
    pub is_default: bool,
    /// A Bluetooth headset (call mode lowers its quality).
    pub bluetooth: bool,
}

/// The pill's rectangle inside the panel window, in logical points.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PanelRegion {
    /// Left.
    pub x: f64,
    /// Top.
    pub y: f64,
    /// Width.
    pub width: f64,
    /// Height.
    pub height: f64,
}

/// Records from the microphone named `name` from now on (`null`: the
/// system default), without a gap in the meeting. Fails with `conflict`
/// when nothing is recording.
#[tauri::command]
#[specta::specta]
pub(super) async fn set_microphone(
    recording: State<'_, Arc<RecordingService>>,
    name: Option<String>,
) -> Result<RecordingSnapshot, AppError> {
    recording.set_microphone(name).into_ipc()
}

/// Records the call's audio, or stops and records silence on its track.
#[tauri::command]
#[specta::specta]
pub(super) async fn set_system_audio(
    recording: State<'_, Arc<RecordingService>>,
    enabled: bool,
) -> Result<RecordingSnapshot, AppError> {
    recording.set_system_audio(enabled).into_ipc()
}

/// The microphones this Mac has, the default first.
#[tauri::command]
#[specta::specta]
pub(super) async fn microphones() -> Result<Vec<Microphone>, AppError> {
    let listed = tauri::async_runtime::spawn_blocking(|| CpalInputs.inputs())
        .await
        .map_err(|_| AppError::new(ErrorCode::Internal, "could not list the microphones"))?;
    let mut microphones: Vec<Microphone> = listed
        .into_iter()
        .filter(|device| !is_loopback(&device.name))
        .map(|device| Microphone {
            bluetooth: device.transport == Transport::Bluetooth,
            is_default: device.is_default,
            name: device.name,
        })
        .collect();
    microphones.sort_by_key(|microphone| !microphone.is_default);
    Ok(microphones)
}

/// "Keep recording": withdraws the "Ending…" notice (CAP-07).
#[tauri::command]
#[specta::specta]
pub(super) async fn keep_recording(
    autostop: State<'_, Arc<AutoStopService>>,
) -> Result<AutoStopState, AppError> {
    autostop.keep_recording();
    Ok(autostop.state())
}

/// Answers "Start new note for …?": `accept` ends this note and starts one
/// for the meeting that began; otherwise the offer goes away.
#[tauri::command]
#[specta::specta]
pub(super) async fn answer_split(
    autostop: State<'_, Arc<AutoStopService>>,
    accept: bool,
) -> Result<AutoStopState, AppError> {
    if accept {
        autostop.accept_split();
    } else {
        autostop.dismiss_split();
    }
    Ok(autostop.state())
}

/// The "Ending…" notice and the offer on screen, for a panel that opens late.
#[tauri::command]
#[specta::specta]
pub(super) async fn auto_stop(
    autostop: State<'_, Arc<AutoStopService>>,
) -> Result<AutoStopState, AppError> {
    Ok(autostop.state())
}

/// Brings the main window forward on the live meeting's notes ("Open
/// notes", or a click on the panel). Fails with `conflict` when nothing is
/// recording.
#[tauri::command]
#[specta::specta]
pub(super) async fn open_live_meeting(
    window: State<'_, Arc<dyn MainWindow>>,
    recording: State<'_, Arc<RecordingService>>,
) -> Result<(), AppError> {
    let Some(meeting_id) = recording.state().meeting_id else {
        return Err(AppError::new(
            ErrorCode::Conflict,
            "nothing is being recorded",
        ));
    };
    window.open_live_meeting(meeting_id);
    Ok(())
}

/// The main window, as the panel's commands reach it.
pub trait MainWindow: Send + Sync {
    /// Brings the main window forward on the live meeting's notes.
    fn open_live_meeting(&self, meeting_id: MeetingId);
}

/// [`MainWindow`] over Tauri.
pub struct TauriMainWindow<R: Runtime>(pub AppHandle<R>);

impl<R: Runtime> MainWindow for TauriMainWindow<R> {
    fn open_live_meeting(&self, meeting_id: MeetingId) {
        crate::tray::focus_main_window(&self.0);
        if let Err(error) = (OpenLiveMeeting { meeting_id }).emit(&self.0) {
            log::warn!(target: "recording", "could not emit open-live-meeting: {error}");
        }
    }
}

/// Where the pill is inside the panel window: the only part that takes the
/// mouse; clicks anywhere else pass through to the app below. Answers with
/// the screen edge the panel hugs, which the pill lays itself out against.
#[tauri::command]
#[specta::specta]
pub(super) async fn set_panel_region(
    host: State<'_, Arc<PanelHost>>,
    region: PanelRegion,
) -> Result<PanelEdge, AppError> {
    let rect = Rect {
        x: region.x,
        y: region.y,
        width: region.width,
        height: region.height,
    };
    Ok(panel::set_region(&host, rect))
}
