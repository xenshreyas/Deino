/**
 * LEGACY SHIM. Answers the untyped `invoke('…')` calls that the inherited
 * screens still make, just enough for the app to run in a plain browser past
 * onboarding (`pnpm dev:mock`).
 *
 * This file only shrinks: when a card ports a legacy command into a typed
 * feature plugin, its mock moves to `src/dev/mock-ipc/<feature>.ts` (checked
 * against the generated bindings) and its entry here is deleted. Never add
 * behaviour here that a typed mock could provide. Unknown commands reject,
 * so a screen that needs one shows its error state instead of fake data.
 */
import type { InvokeArgs } from '@tauri-apps/api/core';

import packageJson from '../../../package.json';

import { mockDatabase } from './database';
import { legacyMeetingResponses } from './legacy-meetings';

/** A legacy command's mock: arguments in, response out. */
type LegacyHandler = (args: Record<string, unknown>) => unknown;

/** Settings and status the inherited providers read at startup. */
const STATIC_RESPONSES: Record<string, unknown> = {
  get_onboarding_status: {
    completed: true,
    current_step: 4,
    version: '1.0',
    model_status: { parakeet: 'downloaded', summary: 'downloaded' },
  },
  check_first_launch: false,
  is_recording: false,
  get_recording_state: { is_recording: false, is_paused: false, is_active: false },
  // The Tauri window API the appearance feature calls; a browser has no window to theme.
  'plugin:window|set_theme': null,
  get_recording_meeting_name: null,
  get_transcript_history: [],
  get_transcription_status: { chunks_in_queue: 0, is_processing: false, last_activity_ms: 0 },
  updates_enabled: false,
  has_audio_checkpoints: false,
  get_audio_devices: [
    { name: 'MacBook Pro Microphone', device_type: 'Input' },
    { name: 'MacBook Pro Speakers', device_type: 'Output' },
  ],
  get_default_recordings_folder_path: '~/Movies/Deino',
  get_database_directory: '~/Library/Application Support/com.xenshreyas.deino',
  whisper_get_models_directory: '~/Library/Application Support/com.xenshreyas.deino/models',
  'plugin:app|version': packageJson.version,
  api_list_templates: [
    {
      id: 'standard_meeting',
      name: 'Standard meeting',
      description: 'Summary, decisions, action items',
    },
  ],
  parakeet_has_available_models: true,
  parakeet_get_available_models: [],
  whisper_has_available_models: false,
  whisper_get_available_models: [],
  builtin_ai_list_models: [],
  builtin_ai_is_model_ready: false,
  get_ollama_models: [],
};

/** Every mocked legacy command. */
const HANDLERS: Record<string, LegacyHandler> = {
  ...Object.fromEntries(
    Object.entries(STATIC_RESPONSES).map(([name, value]) => [name, () => value]),
  ),
  ...legacyMeetingResponses(mockDatabase),
};

/** Commands already reported as unmocked, so each warns once. */
const reported = new Set<string>();

/** Answers one legacy command, or rejects the way a failing Rust command does. */
export function handleLegacyInvoke(command: string, args?: InvokeArgs): unknown {
  const handler = HANDLERS[command];
  if (handler) return handler(isRecord(args) ? args : {});
  if (!reported.has(command)) {
    reported.add(command);
    console.warn(`mock IPC: legacy command "${command}" is not mocked`);
  }
  return Promise.reject(new Error(`${command} is not available in dev:mock`));
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}
