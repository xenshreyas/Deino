import type { QueryClient } from '@tanstack/react-query';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import type * as Generated from '@/lib/ipc/generated/transcript';
import { createQueryClient } from '@/lib/ipc';

import { transcriptKeys } from '../queries';
import { MEETING, segment } from '../test-data';
import type { SegmentId, Transcript } from '../types';
import { deleteLine, saveLineText } from './line-edits';

const transcript = vi.hoisted(() => ({
  get: vi.fn<typeof Generated.commands.get>(),
  edit: vi.fn<typeof Generated.commands.edit>(),
  delete: vi.fn<typeof Generated.commands.delete>(),
  restore: vi.fn<typeof Generated.commands.restore>(),
  previewReplace: vi.fn<typeof Generated.commands.previewReplace>(),
  replace: vi.fn<typeof Generated.commands.replace>(),
}));
vi.mock('@/lib/ipc/generated/transcript', () => ({ commands: transcript }));

const toasts = vi.hoisted(() => ({
  shown: [] as { message: string; undo?: () => void }[],
}));
vi.mock('sonner', () => {
  const toast = (message: string, options?: { action?: { onClick: () => void } }) => {
    toasts.shown.push({ message, ...(options?.action ? { undo: options.action.onClick } : {}) });
  };
  toast.error = (message: string) => toasts.shown.push({ message });
  toast.success = (message: string) => toasts.shown.push({ message });
  return { toast };
});

const LINE = 'line-a' as SegmentId;

let client: QueryClient;

function cached(): string[] {
  const data = client.getQueryData<Transcript>(transcriptKeys.meeting(MEETING));
  return (data?.segments ?? []).map((one) => one.text);
}

beforeEach(() => {
  client = createQueryClient();
  client.setQueryData(transcriptKeys.meeting(MEETING), {
    segments: [segment('line-a', 0, 'Ship Monday', null), segment('line-b', 1, 'Okay', null)],
  });
  toasts.shown = [];
  for (const command of Object.values(transcript)) command.mockReset();
});

describe('saveLineText', () => {
  it('shows the trimmed text at once and saves it', async () => {
    transcript.edit.mockResolvedValue({
      status: 'ok',
      data: segment('line-a', 0, 'Ship Friday', null),
    });

    const saved = await saveLineText(
      { client, meetingId: MEETING, segmentId: LINE },
      ' Ship Friday ',
    );

    expect(saved).toBe(true);
    expect(transcript.edit).toHaveBeenCalledWith(LINE, 'Ship Friday');
    expect(cached()[0]).toBe('Ship Friday');
  });

  it('puts the old text back and says so when Rust refuses', async () => {
    transcript.edit.mockResolvedValue({
      status: 'error',
      error: { code: 'internal', message: 'x' },
    });

    const saved = await saveLineText(
      { client, meetingId: MEETING, segmentId: LINE },
      'Ship Friday',
    );

    expect(saved).toBe(false);
    expect(cached()[0]).toBe('Ship Monday');
    expect(toasts.shown.map((one) => one.message)).toEqual(['Your edit could not be saved']);
  });

  it('never sends a blank line', async () => {
    expect(await saveLineText({ client, meetingId: MEETING, segmentId: LINE }, '   ')).toBe(false);
    expect(transcript.edit).not.toHaveBeenCalled();
  });
});

describe('deleteLine', () => {
  it('removes the line at once and offers Undo, which restores it', async () => {
    transcript.delete.mockResolvedValue({ status: 'ok', data: null });
    transcript.restore.mockResolvedValue({ status: 'ok', data: null });

    await deleteLine({ client, meetingId: MEETING, segmentId: LINE });
    expect(cached()).toEqual(['Okay']);
    toasts.shown[0]?.undo?.();

    expect(toasts.shown[0]?.message).toBe('Line deleted');
    await vi.waitFor(() => {
      expect(transcript.restore).toHaveBeenCalledWith(LINE);
    });
  });

  it('brings the line back when the delete fails', async () => {
    transcript.delete.mockResolvedValue({
      status: 'error',
      error: { code: 'internal', message: 'x' },
    });

    await deleteLine({ client, meetingId: MEETING, segmentId: LINE });

    expect(cached()).toEqual(['Ship Monday', 'Okay']);
    expect(toasts.shown.map((one) => one.message)).toEqual(['The line could not be deleted']);
  });
});
