import { QueryClientProvider } from '@tanstack/react-query';
import type { EventCallback } from '@tauri-apps/api/event';
import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import type * as Generated from '@/lib/ipc/generated/pipeline';
import { createQueryClient } from '@/lib/ipc';

import { copy } from '../copy';
import type { JobStatus, MeetingId, StageId, StageStatus } from '../types';
import { PipelineStrip } from './pipeline-strip';

// The generated bindings, mocked with their real types: a Rust signature
// change makes this file fail to compile instead of silently passing.
const bindings = vi.hoisted(() => {
  const progress: { handler?: EventCallback<Generated.PipelineProgress> } = {};
  const commands = {
    status: vi.fn<typeof Generated.commands.status>(),
    rerun: vi.fn<typeof Generated.commands.rerun>(),
  } satisfies typeof Generated.commands;
  const listen = vi.fn((handler: EventCallback<Generated.PipelineProgress>) => {
    progress.handler = handler;
    return Promise.resolve(vi.fn());
  });
  return { commands, events: { pipelineProgress: { listen } }, progress };
});
vi.mock('@/lib/ipc/generated/pipeline', () => ({
  commands: bindings.commands,
  events: bindings.events,
}));

const MEETING = 'meeting-1' as MeetingId;
const SUMMARY = 'legacy_summary' as StageId;

function stage(status: JobStatus, message: string | null = null): StageStatus {
  return {
    stage: SUMMARY,
    status,
    attempts: 1,
    retryAt: null,
    message,
    updatedAt: '2026-10-09T10:00:00Z',
  };
}

function renderStrip() {
  return render(
    <QueryClientProvider client={createQueryClient()}>
      <PipelineStrip meetingId={MEETING} />
    </QueryClientProvider>,
  );
}

function announce(meetingId: string, status: JobStatus) {
  act(() => {
    const payload = { meetingId: meetingId as MeetingId, stage: SUMMARY, status };
    bindings.progress.handler?.({ event: 'plugin:pipeline:pipeline-progress', id: 1, payload });
  });
}

describe('PipelineStrip', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    bindings.commands.status.mockResolvedValue({ status: 'ok', data: [] });
  });

  it('shows nothing for a meeting the pipeline never ran for', async () => {
    const { container } = renderStrip();

    await waitFor(() => {
      expect(bindings.commands.status).toHaveBeenCalledWith(MEETING);
    });
    expect(container.textContent).toBe('');
  });

  it('shows a failed stage with its reason and retries it alone', async () => {
    const failed = stage('failed', 'Choose a summary model in Settings, then retry.');
    bindings.commands.status.mockResolvedValue({ status: 'ok', data: [failed] });
    bindings.commands.rerun.mockResolvedValue({ status: 'ok', data: [stage('pending')] });
    renderStrip();

    expect(await screen.findByText('Choose a summary model in Settings, then retry.')).toBeDefined();
    fireEvent.click(screen.getByRole('button', { name: copy.retryStage('Summary') }));

    expect(await screen.findByText('Retrying soon')).toBeDefined();
    expect(bindings.commands.rerun).toHaveBeenCalledWith(MEETING, SUMMARY);
    expect(screen.queryByRole('button')).toBeNull();
  });

  it('refetches when Rust reports progress for this meeting only', async () => {
    bindings.commands.status.mockResolvedValue({ status: 'ok', data: [stage('running')] });
    renderStrip();
    expect(await screen.findByText('Running')).toBeDefined();
    bindings.commands.status.mockResolvedValue({ status: 'ok', data: [stage('succeeded')] });

    announce('meeting-2', 'succeeded');
    announce(MEETING, 'succeeded');

    expect(await screen.findByText('Done')).toBeDefined();
    expect(bindings.commands.status).toHaveBeenCalledTimes(2);
  });

  it('says so when a retry is refused', async () => {
    bindings.commands.status.mockResolvedValue({ status: 'ok', data: [stage('cancelled')] });
    bindings.commands.rerun.mockResolvedValue({
      status: 'error',
      error: { code: 'conflict', message: 'meeting meeting-1 cannot be processed while it is recording' },
    });
    renderStrip();

    fireEvent.click(await screen.findByRole('button', { name: copy.retryStage('Summary') }));

    expect((await screen.findByRole('alert')).textContent).toBe(
      'meeting meeting-1 cannot be processed while it is recording',
    );
  });
});
