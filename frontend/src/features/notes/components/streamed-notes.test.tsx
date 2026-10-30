import { QueryClientProvider } from '@tanstack/react-query';
import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import type { ReactNode } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import type * as Generated from '@/lib/ipc/generated/enhance';
import type * as Pipeline from '@/lib/ipc/generated/pipeline';
import { createQueryClient } from '@/lib/ipc';
import { TooltipProvider } from '@/ui/tooltip';

import { notesCopy } from '../copy';
import { TEST_MEETING, testDoc, testVersion, testView } from '../test-notes';
import type { NotesDraft, NotesRun } from '../types';
import { EnhancedNotes } from './enhanced-notes';
import { runStatusText } from './run-status';

// The generated bindings, mocked with their real types; `emit` plays Rust.
const enhance = vi.hoisted(() => ({
  get: vi.fn<typeof Generated.commands.get>(),
  versions: vi.fn<typeof Generated.commands.versions>(),
  enhance: vi.fn<typeof Generated.commands.enhance>(),
  save: vi.fn<typeof Generated.commands.save>(),
  restore: vi.fn<typeof Generated.commands.restore>(),
  cited: vi.fn<typeof Generated.commands.cited>(),
  draft: vi.fn<typeof Generated.commands.draft>(),
  stop: vi.fn<typeof Generated.commands.stop>(),
}));
const runs = vi.hoisted(() => ({ handlers: [] as ((event: { payload: unknown }) => void)[] }));
vi.mock('@/lib/ipc/generated/enhance', () => ({
  commands: enhance,
  events: {
    notesRun: {
      listen: (handler: (event: { payload: unknown }) => void) => {
        runs.handlers.push(handler);
        return Promise.resolve(() => undefined);
      },
    },
  },
}));

const pipeline = vi.hoisted(() => ({
  commands: {
    status: vi.fn<typeof Pipeline.commands.status>(),
    rerun: vi.fn<typeof Pipeline.commands.rerun>(),
  },
  events: { pipelineProgress: { listen: () => Promise.resolve(() => undefined) } },
}));
vi.mock('@/lib/ipc/generated/pipeline', () => pipeline);

function wrapper({ children }: { children: ReactNode }) {
  return (
    <QueryClientProvider client={createQueryClient()}>
      <TooltipProvider>{children}</TooltipProvider>
    </QueryClientProvider>
  );
}

function emit(update: NotesRun['update']) {
  const run: NotesRun = { meetingId: TEST_MEETING, update };
  act(() => {
    runs.handlers.forEach((handler) => {
      handler({ payload: run });
    });
  });
}

/** A draft of the test notes' first `blocks` blocks. */
function draft(blocks: number, overrides: Partial<NotesDraft> = {}): NotesDraft {
  const doc = testDoc();
  return {
    meetingId: TEST_MEETING,
    phase: { kind: 'reading' },
    doc: { type: 'doc', content: doc.content.slice(0, blocks) },
    lines: blocks,
    stale: false,
    ...overrides,
  };
}

describe('streamed notes', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    runs.handlers.length = 0;
    pipeline.commands.status.mockResolvedValue({ status: 'ok', data: [] });
    enhance.versions.mockResolvedValue({ status: 'ok', data: [testVersion(1), testVersion(2)] });
    enhance.get.mockResolvedValue({ status: 'ok', data: null });
    enhance.draft.mockResolvedValue({ status: 'ok', data: null });
    enhance.stop.mockResolvedValue({ status: 'ok', data: true });
  });

  it('shows each finished line as it arrives, says where it is, and stops', async () => {
    render(<EnhancedNotes meetingId={TEST_MEETING} />, { wrapper });
    await screen.findByText(notesCopy.emptyTitle);

    emit({ kind: 'draft', draft: draft(2) });

    const line = await screen.findByText('Maya comes off the on-call rotation.', { exact: false });
    expect(line.closest('[data-origin]')?.getAttribute('data-origin')).toBe('ai');
    expect(screen.getByText(notesCopy.writingSection('Overview'))).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: new RegExp(notesCopy.stop) }));
    await waitFor(() => {
      expect(enhance.stop).toHaveBeenCalledWith(TEST_MEETING);
    });
    emit({ kind: 'ended', end: { kind: 'stopped' } });
    expect(await screen.findByText(notesCopy.stoppedNothing)).toBeTruthy();
    expect(screen.queryByText('Maya comes off the on-call rotation.', { exact: false })).toBeNull();
  });

  it('keeps the old notes until the first new line, then shows the saved version', async () => {
    enhance.get.mockResolvedValue({
      status: 'ok',
      data: testView({ version: testVersion(1), doc: testDoc() }),
    });
    render(<EnhancedNotes meetingId={TEST_MEETING} />, { wrapper });
    await screen.findByText('send her PM draft thurs');

    emit({ kind: 'draft', draft: draft(0) });
    expect(await screen.findByText(notesCopy.readingTranscript)).toBeTruthy();
    expect(screen.getByText('send her PM draft thurs')).toBeTruthy();

    emit({ kind: 'draft', draft: draft(2) });
    await waitFor(() => {
      expect(screen.queryByText('send her PM draft thurs')).toBeNull();
    });

    enhance.get.mockResolvedValue({ status: 'ok', data: testView({ version: testVersion(2) }) });
    emit({ kind: 'ended', end: { kind: 'written', version: 2, kept: 1 } });
    await waitFor(() => {
      expect(screen.queryByText(notesCopy.readingTranscript)).toBeNull();
    });
    expect(screen.getByText('Maya comes off the on-call rotation.', { exact: false })).toBeTruthy();
    expect(screen.queryByText('send her PM draft thurs')).toBeNull();
  });

  it('reads the run in progress when the page opens mid-run', async () => {
    enhance.draft.mockResolvedValue({ status: 'ok', data: draft(2) });
    render(<EnhancedNotes meetingId={TEST_MEETING} />, { wrapper });

    expect(
      await screen.findByText('Maya comes off the on-call rotation.', { exact: false }),
    ).toBeTruthy();
  });
});

describe('runStatusText', () => {
  it('says what the run is doing', () => {
    expect(runStatusText(draft(0))).toBe(notesCopy.readingTranscript);
    expect(runStatusText(draft(4))).toBe(notesCopy.writingSection('Your notes'));
    expect(runStatusText(draft(2, { stale: true, phase: { kind: 'retrying' } }))).toBe(
      notesCopy.retrying,
    );
    expect(runStatusText(draft(0, { phase: { kind: 'parts', done: 1, total: 3 } }))).toBe(
      notesCopy.readingParts(1, 3),
    );
    expect(runStatusText(draft(0, { phase: { kind: 'merging' } }))).toBe(notesCopy.merging);
    expect(runStatusText(draft(5, { phase: { kind: 'saving' } }))).toBe(notesCopy.saving);
  });
});
