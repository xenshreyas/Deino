/**
 * The bridge between the typed settings engine and the provider names the
 * legacy transcription screens still use, until FND-11 ports them.
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

/** The settings engine for a legacy provider name; unknown names get the default. */
export function engineOf(provider: string): TranscriptionEngine {
  if (provider === PROVIDERS.whisper) return 'whisper';
  if (provider === PROVIDERS.parakeet) return 'parakeet';
  return 'parakeet_unified';
}
