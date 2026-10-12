import { QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, test, vi } from 'vitest';

import type * as Cloud from '@/lib/ipc/generated/cloud-transcription';
import { createQueryClient } from '@/lib/ipc';

import type { MeetingId, StageStatus } from '../types';
import { CloudRetry, isCloudFailure } from './cloud-retry';

const cloud = vi.hoisted(() => ({
  retranscribe: vi.fn<typeof Cloud.commands.retranscribe>(),
}));
vi.mock('@/lib/ipc/generated/cloud-transcription', () => ({ commands: cloud }));

const MEETING = 'meeting-a' as MeetingId;

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

const FAILED = step('skipped', 'ElevenLabs transcription failed: the API key was refused.');

describe('Retry beside a failed ElevenLabs transcript', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  test('only the transcript step that gave up on ElevenLabs offers it', () => {
    expect(isCloudFailure(FAILED)).toBe(true);
    expect(isCloudFailure(step('skipped', 'Nothing was said in the recording.'))).toBe(false);
    expect(isCloudFailure(step('succeeded', null))).toBe(false);
  });

  test('retrying sends the meeting to ElevenLabs again', async () => {
    cloud.retranscribe.mockResolvedValue({ status: 'ok', data: [step('pending', null)] });
    render(
      <QueryClientProvider client={createQueryClient()}>
        <CloudRetry meetingId={MEETING} stage={FAILED} />
      </QueryClientProvider>,
    );

    fireEvent.click(screen.getByRole('button', { name: 'Retry ElevenLabs transcription' }));

    await waitFor(() => {
      expect(cloud.retranscribe).toHaveBeenCalledWith(MEETING);
    });
  });
});
