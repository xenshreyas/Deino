import { QueryClientProvider } from '@tanstack/react-query';
import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  RouterProvider,
} from '@tanstack/react-router';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { useState } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import type * as Generated from '@/lib/ipc/generated/enhance';
import { createQueryClient } from '@/lib/ipc';
import { TooltipProvider } from '@/ui/tooltip';

import { notesCopy } from '../copy';
import { CITED_SEGMENT, TEST_MEETING } from '../test-notes';
import type { CitationRef, SegmentId } from '../types';
import { citedBlockAt } from './cited-block';
import { CitationPopover } from './citation-popover';

const enhance = vi.hoisted(() => ({
  get: vi.fn<typeof Generated.commands.get>(),
  versions: vi.fn<typeof Generated.commands.versions>(),
  enhance: vi.fn<typeof Generated.commands.enhance>(),
  save: vi.fn<typeof Generated.commands.save>(),
  restore: vi.fn<typeof Generated.commands.restore>(),
  cited: vi.fn<typeof Generated.commands.cited>(),
}));
vi.mock('@/lib/ipc/generated/enhance', () => ({ commands: enhance }));

const player = vi.hoisted(() => ({
  ready: false,
  playFrom: vi.fn<(meeting: string, ms: number) => void>(),
}));
vi.mock('@/features/player', () => ({
  usePlayerReady: () => player.ready,
  playerActions: { playFrom: player.playFrom, seek: vi.fn() },
}));

const SECOND = '0192f3a0-6c1e-7b2a-9d4e-000000000002';

const CITATIONS: CitationRef[] = [
  { segmentId: CITED_SEGMENT, startMs: 37_000 },
  { segmentId: SECOND, startMs: 248_000 },
];

/** The popover, controlled like `CitedFrame` controls it. */
function Harness() {
  const [open, setOpen] = useState(false);
  return (
    <CitationPopover
      meetingId={TEST_MEETING}
      citations={CITATIONS}
      top={0}
      open={open}
      onOpenChange={setOpen}
    />
  );
}

/** The harness on the meeting route, so "Show in transcript" can navigate. */
function renderPopover() {
  const root = createRootRoute();
  const meeting = createRoute({
    getParentRoute: () => root,
    path: '/meetings/$meetingId',
    component: Harness,
  });
  const router = createRouter({
    routeTree: root.addChildren([meeting]),
    history: createMemoryHistory({ initialEntries: [`/meetings/${TEST_MEETING}`] }),
  });
  render(
    <QueryClientProvider client={createQueryClient()}>
      <TooltipProvider>
        <RouterProvider router={router} />
      </TooltipProvider>
    </QueryClientProvider>,
  );
  return router;
}

describe('CitationPopover', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    enhance.cited.mockResolvedValue({
      status: 'ok',
      data: [
        {
          segmentId: CITED_SEGMENT as SegmentId,
          start: 37_000,
          end: 41_000,
          speaker: 'Shreyas',
          text: 'You come off on-call.',
        },
        {
          segmentId: SECOND as SegmentId,
          start: 248_000,
          end: 250_000,
          speaker: 'Maya',
          text: 'I am off Friday.',
        },
      ],
    });
  });

  it('opens on the magnifier, a real button, with every excerpt, its speaker and time', async () => {
    renderPopover();

    const trigger = await screen.findByRole('button', { name: notesCopy.sourcesLabel });
    expect(trigger.tagName).toBe('BUTTON');
    fireEvent.click(trigger);

    expect(await screen.findByText('You come off on-call.')).toBeTruthy();
    expect(screen.getByText('I am off Friday.')).toBeTruthy();
    expect(screen.getByText('Shreyas')).toBeTruthy();
    expect(screen.getByText('0:37')).toBeTruthy();
    expect(screen.getByText('4:08')).toBeTruthy();
    expect(enhance.cited).toHaveBeenCalledWith(TEST_MEETING, [CITED_SEGMENT, SECOND]);
  });

  it('disables Play without a recording in the player', async () => {
    player.ready = false;
    renderPopover();
    fireEvent.click(await screen.findByRole('button', { name: notesCopy.sourcesLabel }));
    await screen.findByText('You come off on-call.');

    for (const play of screen.getAllByRole('button', { name: notesCopy.noRecording })) {
      expect((play as HTMLButtonElement).disabled).toBe(true);
    }
  });

  it('plays each excerpt from its cited moment', async () => {
    player.ready = true;
    renderPopover();
    fireEvent.click(await screen.findByRole('button', { name: notesCopy.sourcesLabel }));
    await screen.findByText('I am off Friday.');

    const plays = screen.getAllByRole('button', { name: notesCopy.play });
    fireEvent.click(plays[1] ?? plays[0] ?? document.body);

    expect(player.playFrom).toHaveBeenCalledWith(TEST_MEETING, 248_000);
  });

  it('shows the excerpt in the transcript', async () => {
    const router = renderPopover();
    fireEvent.click(await screen.findByRole('button', { name: notesCopy.sourcesLabel }));
    await screen.findByText('You come off on-call.');

    const [first] = screen.getAllByRole('button', { name: notesCopy.showInTranscript });
    if (first === undefined) throw new Error('no Show in transcript button');
    fireEvent.click(first);

    await waitFor(() => {
      expect(router.state.location.search).toMatchObject({
        tab: 'transcript',
        segment: CITED_SEGMENT,
      });
    });
  });

  it('says so when the cited transcript was deleted', async () => {
    enhance.cited.mockResolvedValue({ status: 'ok', data: [] });
    renderPopover();
    fireEvent.click(await screen.findByRole('button', { name: notesCopy.sourcesLabel }));

    expect(await screen.findByText(notesCopy.sourcesGone)).toBeTruthy();
  });
});

describe('citedBlockAt', () => {
  /** Notes with a cited bullet (and a cited sub-point) and a plain one. */
  function notes(): HTMLElement {
    const root = document.createElement('div');
    root.innerHTML = [
      '<ul><li><p>Ship it <span data-segment-id="a" data-start-ms="1000">0:01</span>',
      '<span data-segment-id="a" data-start-ms="1000">0:01</span></p>',
      '<ul><li><p>detail <span data-segment-id="b" data-start-ms="2000">0:02</span></p></li></ul></li>',
      '<li><p>plain</p></li></ul>',
    ].join('');
    return root;
  }

  it('finds a line its own citations, once each, not its sub-points', () => {
    const root = notes();
    const text = root.querySelector('p')?.firstChild ?? null;

    expect(citedBlockAt(text, root)?.citations).toEqual([{ segmentId: 'a', startMs: 1000 }]);
  });

  it('has nothing for a line without sources or outside the notes', () => {
    const root = notes();
    const plain = root.querySelectorAll('p')[2]?.firstChild ?? null;

    expect(citedBlockAt(plain, root)).toBeNull();
    expect(citedBlockAt(document.body, root)).toBeNull();
  });
});
