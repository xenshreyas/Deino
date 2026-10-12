/**
 * The provider names the legacy retranscribe dialog still uses, for a
 * settings engine. One way only: Settings → Transcription reads and writes
 * `settings.transcription` itself, so no legacy name is ever turned back
 * into an engine (and no unknown name can turn into a default).
 */
import type { TranscriptionEngine } from '@/features/settings';

import { PARAKEET_UNIFIED_PROVIDER } from './api';

const PROVIDERS = {
  parakeet_unified: PARAKEET_UNIFIED_PROVIDER,
  parakeet: 'parakeet',
  whisper: 'localWhisper',
} as const satisfies Record<TranscriptionEngine, string>;

/** A provider name as the legacy screens spell it. */
export type TranscriptProviderName = (typeof PROVIDERS)[TranscriptionEngine];

/** The legacy provider name of a settings `engine`. */
export function providerOf(engine: TranscriptionEngine): TranscriptProviderName {
  return PROVIDERS[engine];
}
