import { QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, test, vi } from 'vitest';

import type * as Generated from '@/lib/ipc/generated/vocabulary';
import { createQueryClient } from '@/lib/ipc';
import { TooltipProvider } from '@/ui/tooltip';

import type { CorrectedSegment, MeetingId, SegmentId } from '../types';
import { MeetingCorrections } from './meeting-corrections';

const commands = vi.hoisted(
  () =>
    ({
      list: vi.fn<typeof Generated.commands.list>(),
      create: vi.fn<typeof Generated.commands.create>(),
      update: vi.fn<typeof Generated.commands.update>(),
      delete: vi.fn<typeof Generated.commands.delete>(),
      corrections: vi.fn<typeof Generated.commands.corrections>(),
      undo: vi.fn<typeof Generated.commands.undo>(),
    }) satisfies typeof Generated.commands,
);
vi.mock('@/lib/ipc/generated/vocabulary', () => ({ commands }));

const MEETING = 'meeting-1' as MeetingId;
const thanks: CorrectedSegment = {
  segmentId: 'segment-1' as SegmentId,
  originalText: 'Thanks Sreyas, see you.',
  correctedText: 'Thanks Shreyas, see you.',
  undoable: true,
};
const edited: CorrectedSegment = {
  segmentId: 'segment-2' as SegmentId,
  originalText: 'gonna ship',
  correctedText: 'going to ship',
  undoable: false,
};

function renderCorrections() {
  return render(
    <QueryClientProvider client={createQueryClient()}>
      <TooltipProvider>
        <MeetingCorrections meetingId={MEETING} />
      </TooltipProvider>
    </QueryClientProvider>,
  );
}

describe("a meeting's vocabulary corrections", () => {
  beforeEach(() => {
    vi.resetAllMocks();
    commands.corrections.mockResolvedValue({ status: 'ok', data: [thanks, edited] });
    commands.undo.mockResolvedValue({ status: 'ok', data: { restored: 1, kept: 0 } });
  });

  test('show only the words that changed, and which lines were edited since', async () => {
    renderCorrections();

    expect(await screen.findByText('Sreyas,')).toBeDefined();
    expect(screen.getByText('Shreyas,')).toBeDefined();
    expect(screen.getByText('Edited since')).toBeDefined();
    expect(screen.getByText('Vocabulary · 2 lines corrected')).toBeDefined();
  });

  test('undo one line, or all of them', async () => {
    renderCorrections();

    fireEvent.click(await screen.findByRole('button', { name: 'Undo Sreyas, to Shreyas,' }));
    fireEvent.click(screen.getByRole('button', { name: 'Undo all' }));

    await waitFor(() => {
      expect(commands.undo).toHaveBeenCalledWith(MEETING, 'segment-1');
      expect(commands.undo).toHaveBeenCalledWith(MEETING, null);
    });
  });

  test('render nothing when nothing was corrected', async () => {
    commands.corrections.mockResolvedValue({ status: 'ok', data: [] });

    const { container } = renderCorrections();

    await waitFor(() => {
      expect(commands.corrections).toHaveBeenCalled();
    });
    expect(container.textContent).toBe('');
  });
});
