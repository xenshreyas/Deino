import { describe, expect, test } from 'vitest';

import {
  getProviderCommands,
  hasDownloadingModel,
} from './transcription-model-readiness';

describe('transcription model readiness', () => {
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
