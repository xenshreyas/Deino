import { QueryClientProvider } from '@tanstack/react-query';
import type { EventCallback } from '@tauri-apps/api/event';
import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import type { ReactNode } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import type * as Generated from '@/lib/ipc/generated/pipeline';
import { createQueryClient } from '@/lib/ipc';

import { copy, KNOWN_STAGES, stageFailed, stageLabel, stageRunning } from '../copy';
import type { JobStatus, MeetingId, StageId, StageStatus } from '../types';
import { PipelineStatus } from './pipeline-status';
import { ProcessingDetailsDialog } from './processing-details';

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
const NOTES = 'enhance_notes' as StageId;

function stage(status: JobStatus, message: string | null = null, id: StageId = NOTES): StageStatus {
  return {
    stage: id,
    status,
    attempts: 1,
    retryAt: null,
    message,
    updatedAt: '2026-10-09T10:00:00Z',
  };
}

function wrap(children: ReactNode) {
  return render(<QueryClientProvider client={createQueryClient()}>{children}</QueryClientProvider>);
}

function stages(list: StageStatus[]) {
  bindings.commands.status.mockResolvedValue({ status: 'ok', data: list });
}

function announce(meetingId: string, status: JobStatus) {
  act(() => {
    const payload = { meetingId: meetingId as MeetingId, stage: NOTES, status };
    bindings.progress.handler?.({ event: 'plugin:pipeline:pipeline-progress', id: 1, payload });
  });
}

describe('PipelineStatus', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    stages([]);
  });

  it('says nothing when every stage worked or was skipped', async () => {
    const tags = 'auto_tag' as StageId;
    stages([stage('succeeded'), stage('skipped', 'There are no tags to apply.', tags)]);
    const { container } = wrap(<PipelineStatus meetingId={MEETING} />);

    await waitFor(() => {
      expect(bindings.commands.status).toHaveBeenCalledWith(MEETING);
    });
    expect(container.textContent).toBe('');
  });

  it('shows one quiet line while work runs, and goes quiet when it is done', async () => {
    stages([stage('succeeded', null, 'final_transcription' as StageId), stage('running')]);
    const { container } = wrap(<PipelineStatus meetingId={MEETING} />);
    expect((await screen.findByRole('status')).textContent).toBe('Writing notes…');
    stages([stage('succeeded')]);

    announce('meeting-2', 'succeeded');
    announce(MEETING, 'succeeded');

    await waitFor(() => {
      expect(container.textContent).toBe('');
    });
    expect(bindings.commands.status).toHaveBeenCalledTimes(2);
  });

  it('names a failed stage in plain words, with its reason, and retries it alone', async () => {
    stages([stage('failed', 'Choose an AI model in Settings, then retry.')]);
    bindings.commands.rerun.mockResolvedValue({ status: 'ok', data: [stage('pending')] });
    wrap(<PipelineStatus meetingId={MEETING} />);

    expect(await screen.findByText('Notes couldn’t be written.')).toBeDefined();
    screen.getByText('Choose an AI model in Settings, then retry.');
    fireEvent.click(screen.getByRole('button', { name: copy.retryStage('Notes') }));

    expect(await screen.findByText('Writing notes…')).toBeDefined();
    expect(bindings.commands.rerun).toHaveBeenCalledWith(MEETING, NOTES);
  });

  it('says so when a retry is refused', async () => {
    stages([stage('cancelled')]);
    bindings.commands.rerun.mockResolvedValue({
      status: 'error',
      error: {
        code: 'conflict',
        message: 'meeting meeting-1 cannot be processed while it is recording',
      },
    });
    wrap(<PipelineStatus meetingId={MEETING} />);

    fireEvent.click(await screen.findByRole('button', { name: copy.retryStage('Notes') }));

    expect((await screen.findByRole('alert')).textContent).toBe(
      'meeting meeting-1 cannot be processed while it is recording',
    );
  });
});

describe('ProcessingDetailsDialog', () => {
  beforeEach(() => {
    vi.resetAllMocks();
  });

  it('lists every stage, done and skipped too, with why', async () => {
    const title = 'auto_title' as StageId;
    stages([stage('succeeded'), stage('skipped', 'The meeting already has a title.', title)]);
    wrap(<ProcessingDetailsDialog meetingId={MEETING} open onOpenChange={vi.fn()} />);

    expect(await screen.findByText('Notes')).toBeDefined();
    screen.getByText('Done');
    screen.getByText('Title');
    screen.getByText('The meeting already has a title.');
  });
});

describe('stage words', () => {
  it('names every registered stage in sentence case, with no ID showing', () => {
    for (const id of KNOWN_STAGES) {
      const label = stageLabel(id);
      expect(label).not.toContain('_');
      expect(label.charAt(0)).toBe(label.charAt(0).toUpperCase());
      expect(label.slice(1)).not.toMatch(/ [A-Z]/);
      expect(stageRunning(id)).toMatch(/…$/);
      expect(stageFailed(id)).toMatch(/\.$/);
    }
    expect(stageLabel('a_stage_from_the_future')).toBe('Processing');
  });
});
