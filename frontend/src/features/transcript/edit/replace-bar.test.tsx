import { QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import type * as Generated from '@/lib/ipc/generated/transcript';
import { createQueryClient } from '@/lib/ipc';
import { TooltipProvider } from '@/ui/tooltip';

import { MEETING } from '../test-data';
import { ReplaceBar } from './replace-bar';

const transcript = vi.hoisted(() => ({
  get: vi.fn<typeof Generated.commands.get>(),
  edit: vi.fn<typeof Generated.commands.edit>(),
  delete: vi.fn<typeof Generated.commands.delete>(),
  restore: vi.fn<typeof Generated.commands.restore>(),
  previewReplace: vi.fn<typeof Generated.commands.previewReplace>(),
  replace: vi.fn<typeof Generated.commands.replace>(),
}));
vi.mock('@/lib/ipc/generated/transcript', () => ({ commands: transcript }));

function renderBar(find: string) {
  return render(
    <QueryClientProvider client={createQueryClient()}>
      <TooltipProvider>
        <ReplaceBar meetingId={MEETING} find={find} />
      </TooltipProvider>
    </QueryClientProvider>,
  );
}

beforeEach(() => {
  transcript.previewReplace.mockResolvedValue({
    status: 'ok',
    data: {
      matches: 3,
      segments: 2,
      meetings: [{ meetingId: MEETING, title: 'Standup', matches: 3 }],
    },
  });
  transcript.replace.mockResolvedValue({
    status: 'ok',
    data: { matches: 3, segments: 2, meetings: 1 },
  });
});

describe('ReplaceBar', () => {
  it('counts the matches in this meeting before replacing them', async () => {
    renderBar('dino');
    fireEvent.change(screen.getByRole('textbox', { name: 'Replace with' }), {
      target: { value: 'Deino' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Replace all' }));

    expect(await screen.findByText('3 matches in 2 lines')).toBeTruthy();
    const request = {
      find: 'dino',
      matchCase: false,
      scope: { kind: 'meeting', meetingId: MEETING },
    };
    expect(transcript.previewReplace).toHaveBeenCalledWith(request);
    expect(transcript.replace).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole('button', { name: 'Replace 3 matches' }));

    await vi.waitFor(() => {
      expect(transcript.replace).toHaveBeenCalledWith(request, 'Deino');
    });
  });

  it('asks across every meeting, case-sensitively, when told to', async () => {
    renderBar('Dino');
    fireEvent.click(screen.getByRole('button', { name: 'Match case' }));
    fireEvent.click(screen.getByRole('radio', { name: 'All meetings' }));
    fireEvent.click(screen.getByRole('button', { name: 'Replace all' }));

    expect(await screen.findByText(/across 1 meeting/)).toBeTruthy();
    expect(transcript.previewReplace).toHaveBeenLastCalledWith({
      find: 'Dino',
      matchCase: true,
      scope: { kind: 'all' },
    });
  });

  it('cannot replace before there is something to find', () => {
    renderBar('  ');

    expect(screen.getByRole('button', { name: 'Replace all' }).hasAttribute('disabled')).toBe(true);
  });
});
