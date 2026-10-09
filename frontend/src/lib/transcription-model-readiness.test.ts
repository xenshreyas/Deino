import { describe, expect, test } from 'vitest';

import {
  getProviderCommands,
  hasDownloadingModel,
} from './transcription-model-readiness';

describe('transcription model readiness', () => {
  test('uses Whisper commands when Whisper is the configured provider', () => {
    expect(getProviderCommands('localWhisper')).toEqual({
      initialize: 'whisper_init',
      hasAvailableModels: 'whisper_has_available_models',
      getAvailableModels: 'whisper_get_available_models',
    });
  });

  test('uses Parakeet commands when Parakeet is the configured provider', () => {
    expect(getProviderCommands('parakeet')).toEqual({
      initialize: 'parakeet_init',
      hasAvailableModels: 'parakeet_has_available_models',
      getAvailableModels: 'parakeet_get_available_models',
    });
  });

  test('does not silently treat an unsupported provider as Parakeet', () => {
    expect(getProviderCommands('unsupported-provider')).toBeNull();
  });

  test('recognizes only active downloads', () => {
    expect(hasDownloadingModel([{ status: 'Available' }])).toBe(false);
    expect(hasDownloadingModel([{ status: 'Downloading' }])).toBe(true);
    expect(hasDownloadingModel([{ status: { Downloading: { progress: 0 } } }])).toBe(true);
    expect(hasDownloadingModel([{ status: { Downloading: { progress: 42 } } }])).toBe(true);
  });
});
