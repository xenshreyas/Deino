import { QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import type { ReactNode } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { retentionCopy } from '@/features/retention';
import type * as Generated from '@/lib/ipc/generated/enhance';
import type * as Pipeline from '@/lib/ipc/generated/pipeline';
import type * as Retention from '@/lib/ipc/generated/retention';
import { createQueryClient } from '@/lib/ipc';
import { TooltipProvider } from '@/ui/tooltip';

import { notesCopy } from '../copy';
import { TEST_MEETING, testView } from '../test-notes';
import { EnhanceButton } from './enhance-button';
import { EnhancedNotes } from './enhanced-notes';

// PRV-01: once retention deleted the transcript, nothing regenerates notes.

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

const retention = vi.hoisted(() => ({ status: vi.fn<typeof Retention.commands.status>() }));
vi.mock('@/lib/ipc/generated/retention', () => ({ commands: retention }));

const BLOCKED = retentionCopy.deleted.regenerateBlocked;

function wrapper({ children }: { children: ReactNode }) {
  return (
    <QueryClientProvider client={createQueryClient()}>
      <TooltipProvider>{children}</TooltipProvider>
    </QueryClientProvider>
  );
}

describe('regenerating once the transcript is deleted', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    pipeline.commands.status.mockResolvedValue({ status: 'ok', data: [] });
    enhance.versions.mockResolvedValue({ status: 'ok', data: [] });
    enhance.get.mockResolvedValue({
      status: 'ok',
      data: testView({ reasons: ['transcriptEdited'] }),
    });
    retention.status.mockResolvedValue({
      status: 'ok',
      data: { audioDeletedAt: '2026-10-01T06:00:00Z', transcriptDeletedAt: '2026-10-01T06:00:00Z' },
    });
  });

  it('disables Regenerate and says why', async () => {
    render(<EnhanceButton meetingId={TEST_MEETING} />, { wrapper });

    expect(await screen.findByText(BLOCKED)).toBeTruthy();
    const button = screen.getByRole('button', { name: new RegExp(notesCopy.regenerate) });
    expect(button.hasAttribute('disabled')).toBe(true);
    fireEvent.click(button);
    expect(enhance.enhance).not.toHaveBeenCalled();
  });

  it('hides the regenerate banner, though the transcript changed', async () => {
    render(<EnhancedNotes meetingId={TEST_MEETING} />, { wrapper });

    await waitFor(() => {
      expect(retention.status).toHaveBeenCalledWith(TEST_MEETING);
    });
    await screen.findAllByRole('heading');
    expect(screen.queryByText(notesCopy.bannerLead, { exact: false })).toBeNull();
  });

  it('offers no Enhance when there are no notes yet', async () => {
    enhance.get.mockResolvedValue({ status: 'ok', data: null });
    render(<EnhancedNotes meetingId={TEST_MEETING} />, { wrapper });

    expect(await screen.findByText(BLOCKED)).toBeTruthy();
    const button = screen.getByRole('button', { name: new RegExp(notesCopy.enhance) });
    expect(button.hasAttribute('disabled')).toBe(true);
  });

  it('regenerates as before while the transcript is kept', async () => {
    retention.status.mockResolvedValue({
      status: 'ok',
      data: { audioDeletedAt: null, transcriptDeletedAt: null },
    });
    enhance.enhance.mockResolvedValue({ status: 'ok', data: testView() });
    render(<EnhanceButton meetingId={TEST_MEETING} />, { wrapper });

    await waitFor(() => {
      expect(retention.status).toHaveBeenCalled();
    });
    fireEvent.click(await screen.findByRole('button', { name: new RegExp(notesCopy.regenerate) }));

    await waitFor(() => {
      expect(enhance.enhance).toHaveBeenCalledWith(TEST_MEETING, false);
    });
  });
});
