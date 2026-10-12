import { describe, expect, test } from 'vitest';

import { isCloudFailure } from './cloud-failure';
import type { StageStatus } from './types';

function step(status: StageStatus['status'], message: string | null): StageStatus {
  return {
    stage: 'final_transcription' as StageStatus['stage'],
    status,
    attempts: 1,
    retryAt: null,
    message,
    updatedAt: '2026-10-12T10:00:00Z',
  };
}

describe('a transcript step that gave up on ElevenLabs', () => {
  test('is told apart from other skips and from success', () => {
    const failed = step('skipped', 'ElevenLabs transcription failed: the API key was refused.');

    expect(isCloudFailure(failed)).toBe(true);
    expect(isCloudFailure(step('skipped', 'Nothing was said in the recording.'))).toBe(false);
    expect(isCloudFailure(step('succeeded', null))).toBe(false);
  });
});
