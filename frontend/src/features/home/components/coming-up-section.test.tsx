import { QueryClientProvider } from '@tanstack/react-query';
import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  RouterProvider,
} from '@tanstack/react-router';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import type { MeetingId } from '@/lib/ipc/generated/meetings';
import { createQueryClient } from '@/lib/ipc';
import { TooltipProvider } from '@/ui/tooltip';

import { eventIn, SYNCED } from '../test-home';
import { ComingUpSection } from './coming-up-section';

const bindings = await vi.hoisted(async () =>
  (await import('../test-home')).homeCalendarBindings(),
);
vi.mock('@/lib/ipc/generated/calendar', () => bindings);
const mocked = bindings.commands;

const LAST = 'meeting-last' as MeetingId;

/** Home's Coming up at `/`, with a meeting page to navigate to. */
function renderSection() {
  const root = createRootRoute();
  const home = createRoute({ getParentRoute: () => root, path: '/', component: ComingUpSection });
  const meeting = createRoute({
    getParentRoute: () => root,
    path: '/meetings/$meetingId',
    component: () => <p>Meeting page</p>,
  });
  const router = createRouter({
    routeTree: root.addChildren([home, meeting]),
    history: createMemoryHistory({ initialEntries: ['/'] }),
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

/** Opens the row's menu by right-clicking its title. */
async function rightClick(title: string) {
  fireEvent.contextMenu(await screen.findByText(title));
  return screen.findByRole('menu');
}

describe('Home Coming up', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.useFakeTimers({ toFake: ['Date'], now: new Date(2026, 9, 9, 12, 0) });
    const event = eventIn('Pricing review', 12);
    mocked.status.mockResolvedValue({ status: 'ok', data: SYNCED });
    mocked.listCalendars.mockResolvedValue({ status: 'ok', data: [] });
    mocked.recordPlan.mockResolvedValue({
      status: 'ok',
      data: [{ event, decision: { record: false, reason: 'audienceOff' } }],
    });
    mocked.join.mockResolvedValue({ status: 'ok', data: LAST });
    mocked.lastMeeting.mockResolvedValue({
      status: 'ok',
      data: { id: LAST, title: 'Pricing review', happenedAt: event.startAt, by: 'series' },
    });
    mocked.openInCalendar.mockResolvedValue({ status: 'ok', data: null });
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('joins and records in one call', async () => {
    renderSection();

    fireEvent.click(await screen.findByRole('button', { name: 'Join' }));

    await waitFor(() => {
      expect(mocked.join).toHaveBeenCalledWith('Pricing review', 'joinAndRecord');
    });
  });

  it('opens the row menu on right-click and reviews the last meeting', async () => {
    const router = renderSection();

    const menu = await rightClick('Pricing review');
    fireEvent.click(await screen.findByRole('menuitem', { name: 'Review last meeting' }));

    expect(menu).toBeTruthy();
    await waitFor(() => {
      expect(mocked.lastMeeting).toHaveBeenCalledWith('Pricing review');
    });
    await waitFor(() => {
      expect(router.state.location.pathname).toBe(`/meetings/${LAST}`);
    });
  });

  it('opens the event in Calendar from the menu', async () => {
    renderSection();

    await rightClick('Pricing review');
    fireEvent.click(await screen.findByRole('menuitem', { name: 'Open in Calendar' }));

    await waitFor(() => {
      expect(mocked.openInCalendar).toHaveBeenCalledWith('Pricing review');
    });
  });
});
