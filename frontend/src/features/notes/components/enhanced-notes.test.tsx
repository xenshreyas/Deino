import { QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import type { ReactNode } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import type * as Generated from '@/lib/ipc/generated/enhance';
import type * as Pipeline from '@/lib/ipc/generated/pipeline';
import { createQueryClient } from '@/lib/ipc';
import { TooltipProvider } from '@/ui/tooltip';

import { notesCopy } from '../copy';
import { TEST_MEETING, testVersion, testView } from '../test-notes';
import { EnhanceButton } from './enhance-button';
import { EnhancedNotes } from './enhanced-notes';

// The generated bindings, mocked with their real types.
const enhance = vi.hoisted(() => ({
  get: vi.fn<typeof Generated.commands.get>(),
  versions: vi.fn<typeof Generated.commands.versions>(),
  enhance: vi.fn<typeof Generated.commands.enhance>(),
  save: vi.fn<typeof Generated.commands.save>(),
  restore: vi.fn<typeof Generated.commands.restore>(),
  cited: vi.fn<typeof Generated.commands.cited>(),
  draft: vi.fn<typeof Generated.commands.draft>(() =>
    Promise.resolve({ status: 'ok', data: null }),
  ),
  stop: vi.fn<typeof Generated.commands.stop>(),
}));
vi.mock('@/lib/ipc/generated/enhance', () => ({
  commands: enhance,
  events: { notesRun: { listen: () => Promise.resolve(() => undefined) } },
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

const conflict = {
  status: 'error',
  error: { code: 'conflict', message: 'Regenerating will replace the notes you edited.' },
} as const;

describe('EnhancedNotes', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    pipeline.commands.status.mockResolvedValue({ status: 'ok', data: [] });
    enhance.versions.mockResolvedValue({ status: 'ok', data: [testVersion(1)] });
    enhance.get.mockResolvedValue({ status: 'ok', data: testView() });
  });

  it('offers to write notes when there are none yet', async () => {
    enhance.get.mockResolvedValue({ status: 'ok', data: null });
    enhance.enhance.mockResolvedValue({ status: 'ok', data: testView() });
    render(<EnhancedNotes meetingId={TEST_MEETING} />, { wrapper });

    expect(await screen.findByText(notesCopy.emptyTitle)).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: new RegExp(notesCopy.enhance) }));

    await waitFor(() => {
      expect(enhance.enhance).toHaveBeenCalledWith(TEST_MEETING, false);
    });
  });

  it('renders user text as ink and AI text as grey, with contents for three sections', async () => {
    render(<EnhancedNotes meetingId={TEST_MEETING} />, { wrapper });

    const ai = await screen.findByText('Maya comes off the on-call rotation.', { exact: false });
    expect(ai.closest('[data-origin]')?.getAttribute('data-origin')).toBe('ai');
    const user = screen.getByText('send her PM draft thurs');
    expect(user.closest('[data-origin]')?.getAttribute('data-origin')).toBe('user');
    expect(screen.getByRole('navigation', { name: notesCopy.contents })).toBeTruthy();
  });

  it('shows why the notes may be stale, with Regenerate', async () => {
    enhance.get.mockResolvedValue({
      status: 'ok',
      data: testView({ reasons: ['notesEdited', 'audioAppended'] }),
    });
    render(<EnhancedNotes meetingId={TEST_MEETING} />, { wrapper });

    expect(
      await screen.findByText('you edited your notes and more of the meeting was transcribed', {
        exact: false,
      }),
    ).toBeTruthy();
  });

  it('says the notes could not be opened when loading fails', async () => {
    enhance.get.mockResolvedValue({
      status: 'error',
      error: { code: 'internal', message: 'boom' },
    });
    render(<EnhancedNotes meetingId={TEST_MEETING} />, { wrapper });

    expect(await screen.findByText(notesCopy.loadFailed)).toBeTruthy();
  });
});

describe('EnhanceButton', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    pipeline.commands.status.mockResolvedValue({ status: 'ok', data: [] });
    enhance.get.mockResolvedValue({
      status: 'ok',
      data: testView({ version: testVersion(1, true) }),
    });
  });

  it('asks before replacing edited notes, then regenerates with consent', async () => {
    enhance.enhance.mockImplementation((_, replaceEdits) =>
      Promise.resolve(replaceEdits ? { status: 'ok', data: testView() } : conflict),
    );
    render(<EnhanceButton meetingId={TEST_MEETING} />, { wrapper });

    fireEvent.click(await screen.findByRole('button', { name: new RegExp(notesCopy.regenerate) }));
    const dialog = await screen.findByRole('dialog', { name: notesCopy.confirmTitle });
    fireEvent.click(within(dialog).getByRole('button', { name: notesCopy.confirmAction }));

    await waitFor(() => {
      expect(enhance.enhance).toHaveBeenLastCalledWith(TEST_MEETING, true);
    });
    expect(enhance.enhance).toHaveBeenCalledTimes(2);
  });

  it('keeps the edits when the user cancels', async () => {
    enhance.enhance.mockResolvedValue(conflict);
    render(<EnhanceButton meetingId={TEST_MEETING} />, { wrapper });

    fireEvent.click(await screen.findByRole('button', { name: new RegExp(notesCopy.regenerate) }));
    fireEvent.click(await screen.findByRole('button', { name: notesCopy.cancel }));

    await waitFor(() => {
      expect(screen.queryByRole('dialog')).toBeNull();
    });
    expect(enhance.enhance).toHaveBeenCalledTimes(1);
  });
});
