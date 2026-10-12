import { QueryClientProvider } from '@tanstack/react-query';
import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  RouterProvider,
} from '@tanstack/react-router';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { asMeetingId } from '@/features/meetings';
import type * as Generated from '@/lib/ipc/generated/insights';
import { createQueryClient } from '@/lib/ipc';

import { asInsightId, type MeetingInsight } from '../types';
import { MeetingInsights } from './meeting-insights';

const commands = vi.hoisted(() => ({
  forMeeting: vi.fn<typeof Generated.commands.forMeeting>(),
  run: vi.fn<typeof Generated.commands.run>(),
}));
vi.mock('@/lib/ipc/generated/insights', () => ({ commands }));

const MEETING = asMeetingId('meeting-fixture-sales-discovery');

function entry(stale: boolean): MeetingInsight {
  return {
    def: {
      id: asInsightId('0192f3a0-6c1e-7b2a-9d4e-000000000001'),
      name: 'Discovery scorecard',
      description: 'Score the call.',
      fields: [{ name: 'Budget', kind: { type: 'number' } }],
      cardinality: { kind: 'fixed', count: 1 },
      applyWhen: { mode: 'manual' },
      revision: stale ? 2 : 1,
      createdAt: '2026-10-08T09:00:00Z',
      updatedAt: '2026-10-08T09:00:00Z',
    },
    run: {
      insightId: asInsightId('0192f3a0-6c1e-7b2a-9d4e-000000000001'),
      meetingId: MEETING,
      status: 'done',
      answers: [
        {
          value: {
            kind: 'fields',
            fields: [{ name: 'Budget', value: { type: 'number', value: 40_000 } }],
          },
          speaker: 'Marcus Webb',
          timestampMs: 271_000,
          segmentId: null,
        },
      ],
      revision: 1,
      error: null,
      requestedAt: '2026-10-08T17:00:00Z',
      generatedAt: '2026-10-08T17:00:00Z',
    },
    stale,
  };
}

/** The section on the meeting route, so attribution links can navigate. */
function renderTab() {
  const root = createRootRoute();
  const meeting = createRoute({
    getParentRoute: () => root,
    path: '/meetings/$meetingId',
    component: () => <MeetingInsights meetingId={MEETING} />,
  });
  const router = createRouter({
    routeTree: root.addChildren([meeting]),
    history: createMemoryHistory({ initialEntries: [`/meetings/${MEETING}`] }),
  });
  render(
    <QueryClientProvider client={createQueryClient()}>
      <RouterProvider router={router} />
    </QueryClientProvider>,
  );
  return router;
}

describe("a meeting's insights, at the end of its notes", () => {
  beforeEach(() => {
    vi.resetAllMocks();
  });

  it('shows typed answers and opens the transcript where they were said', async () => {
    commands.forMeeting.mockResolvedValue({
      status: 'ok',
      data: { entries: [entry(false)], others: [] },
    });
    const router = renderTab();

    expect(await screen.findByText('40,000')).toBeTruthy();
    fireEvent.click(screen.getByRole('link', { name: /where Marcus Webb said this, at 04:31/ }));

    await waitFor(() => {
      expect(router.state.location.search).toMatchObject({ tab: 'transcript', t: 271_000 });
    });
  });

  it('shows nothing until an insight has run here', async () => {
    commands.forMeeting.mockResolvedValue({ status: 'ok', data: { entries: [], others: [] } });
    renderTab();

    await waitFor(() => {
      expect(commands.forMeeting).toHaveBeenCalledWith(MEETING);
    });
    expect(screen.queryByRole('region', { name: 'Insights' })).toBeNull();
  });

  it('offers to run a stale insight again on this meeting', async () => {
    commands.forMeeting.mockResolvedValue({
      status: 'ok',
      data: { entries: [entry(true)], others: [] },
    });
    commands.run.mockResolvedValue({ status: 'ok', data: { pending: 1, done: 0, failed: 0 } });
    renderTab();

    fireEvent.click(await screen.findByRole('button', { name: 'Run again' }));

    await waitFor(() => {
      expect(commands.run).toHaveBeenCalledWith(entry(true).def.id, [MEETING]);
    });
  });
});
