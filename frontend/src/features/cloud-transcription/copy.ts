/** Every string the ElevenLabs transcription feature shows (§3A). */
import type { CloudTranscriptionModel } from './types';

/**
 * What the transcript step's status line starts with when ElevenLabs
 * failed and the Mac's transcript was kept; Rust's `CLOUD_FAILED`.
 */
export const CLOUD_FAILED = 'ElevenLabs transcription failed';

export const cloudCopy = {
  title: 'ElevenLabs (optional)',
  description:
    'After each meeting, its audio is sent to ElevenLabs for a more accurate transcript. Live transcription stays on this Mac.',
  card: 'ElevenLabs transcription',
  keyLabel: 'API key',
  keyPlaceholder: 'Your ElevenLabs API key',
  keySaved: 'Saved in your Keychain.',
  keyLocked: 'macOS blocked the saved key. Enter it again.',
  keyChecked: 'ElevenLabs accepted the key.',
  save: 'Save',
  check: 'Check',
  forget: 'Forget',
  modelLabel: 'Model',
  models: {
    scribe_v2: 'Scribe v2',
    scribe_v1: 'Scribe v1',
  } satisfies Record<CloudTranscriptionModel, string>,
  switchLabel: 'Transcribe meetings with ElevenLabs after they end',
  switchNeedsKey: 'Save an API key first.',
  saveFailed: 'Could not save that change.',
  retranscribe: 'Re-transcribe with ElevenLabs',
  restore: 'Restore local transcript',
  retranscribeFailed: 'Could not start ElevenLabs transcription.',
  restoreFailed: 'Could not restore the local transcript.',
  retry: 'Retry',
  retryLabel: 'Retry ElevenLabs transcription',
} as const;
