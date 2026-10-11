//! Commands of the `speakers` plugin: parse input, call one service method,
//! map the error. No logic lives here.

#![allow(
    clippy::let_underscore_must_use,
    reason = "#[tauri::command] on an async fn with a State<'_> argument expands to a `let _` type check of its Result"
)]

use std::sync::Arc;

use deino_core::speakers::{NameCandidate, Participant, Rename, Renamed, SpeakerService};
use deino_core::voiceprints::{VoiceProfile, VoiceprintService};
use deino_domain::{AppError, MeetingId, PersonId, SpeakerId, VoiceprintId};
use tauri::{Runtime, State};

use crate::error::IntoIpc;

/// The speaker naming service, as Tauri state.
type Speakers<'a> = State<'a, Arc<SpeakerService>>;
/// The voice profile service, as Tauri state.
type Voiceprints<'a> = State<'a, Arc<VoiceprintService>>;

/// The plugin's commands, collected here so they can stay private.
pub(super) fn collect<R: Runtime>() -> tauri_specta::Commands<R> {
    tauri_specta::collect_commands![
        participants,
        candidates,
        rename,
        merge,
        confirm_suggestion,
        dismiss_suggestion,
        voice_profiles,
        delete_voiceprint,
        forget_voice,
    ]
}

/// The meeting's speakers, most speech first: colour, name, speaking time
/// and share, and any "Is this …?" suggestion.
#[tauri::command]
#[specta::specta]
async fn participants(
    speakers: Speakers<'_>,
    meeting_id: MeetingId,
) -> Result<Vec<Participant>, AppError> {
    speakers.participants(&meeting_id).await.into_ipc()
}

/// Up to eight people to name a speaker after: matching attendees first.
#[tauri::command]
#[specta::specta]
async fn candidates(
    speakers: Speakers<'_>,
    meeting_id: MeetingId,
    query: String,
) -> Result<Vec<NameCandidate>, AppError> {
    speakers.candidates(&meeting_id, &query).await.into_ipc()
}

/// Names a speaker for one segment or all of them (which enrols the voice);
/// fails with `invalid` for a blank or overlong new name and `not_found` for
/// an unknown speaker, segment or person.
#[tauri::command]
#[specta::specta]
async fn rename(speakers: Speakers<'_>, rename: Rename) -> Result<Renamed, AppError> {
    speakers.rename(&rename).await.into_ipc()
}

/// Folds speaker `from` into `into`; fails with `invalid` for the same
/// speaker twice or for the user's own voice.
#[tauri::command]
#[specta::specta]
async fn merge(
    speakers: Speakers<'_>,
    meeting_id: MeetingId,
    from: SpeakerId,
    into: SpeakerId,
) -> Result<(), AppError> {
    speakers.merge(&meeting_id, from, into).await.into_ipc()
}

/// "Yes": names the speaker after its suggestion for all its segments.
#[tauri::command]
#[specta::specta]
async fn confirm_suggestion(
    speakers: Speakers<'_>,
    meeting_id: MeetingId,
    speaker_id: SpeakerId,
) -> Result<Renamed, AppError> {
    speakers
        .confirm_suggestion(&meeting_id, speaker_id)
        .await
        .into_ipc()
}

/// "No": forgets the speaker's suggestion.
#[tauri::command]
#[specta::specta]
async fn dismiss_suggestion(speakers: Speakers<'_>, speaker_id: SpeakerId) -> Result<(), AppError> {
    speakers.dismiss_suggestion(speaker_id).await.into_ipc()
}

/// Everyone with a voice profile, by name, best exemplar first.
#[tauri::command]
#[specta::specta]
async fn voice_profiles(voiceprints: Voiceprints<'_>) -> Result<Vec<VoiceProfile>, AppError> {
    voiceprints.profiles().await.into_ipc()
}

/// Deletes one exemplar of a voice profile.
#[tauri::command]
#[specta::specta]
async fn delete_voiceprint(voiceprints: Voiceprints<'_>, id: VoiceprintId) -> Result<(), AppError> {
    voiceprints.delete(&id).await.into_ipc()
}

/// Deletes a person's whole voice profile.
#[tauri::command]
#[specta::specta]
async fn forget_voice(voiceprints: Voiceprints<'_>, person_id: PersonId) -> Result<(), AppError> {
    voiceprints.forget(&person_id).await.map(|_| ()).into_ipc()
}
