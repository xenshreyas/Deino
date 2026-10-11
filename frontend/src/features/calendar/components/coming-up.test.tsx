import { QueryClientProvider } from '@tanstack/react-query';
import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  RouterProvider,
} from '@tanstack/react-router';
import { act, fireEvent, render, screen, within } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { createQueryClient } from '@/lib/ipc';

import { meetingIn, status, workCalendar } from '../test-calendar';
import { ComingUp } from './coming-up';

const bindings = await vi.hoisted(async () =>
  (await import('../test-calendar')).calendarBindings(),
);
vi.mock('@/lib/ipc/generated/calendar', () => bindings);
const mocked = bindings.commands;

/** "Coming up" at `/`, with a `/settings` page to navigate to. */
function renderComingUp() {
  const root = createRootRoute();
  const home = createRoute({ getParentRoute: () => root, path: '/', component: ComingUp });
  const settings = createRoute({
    getParentRoute: () => root,
    path: '/settings',
    component: () => <p>Settings page</p>,
  });
  const router = createRouter({
    routeTree: root.addChildren([home, settings]),
    history: createMemoryHistory({ initialEntries: ['/'] }),
  });
  render(
    <QueryClientProvider client={createQueryClient()}>
      <RouterProvider router={router} />
    </QueryClientProvider>,
  );
  return router;
}

describe('ComingUp', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // Local noon: every event below starts today, whatever the real time.
    vi.useFakeTimers({ toFake: ['Date'], now: new Date(2026, 9, 9, 12, 0) });
    mocked.status.mockResolvedValue({ status: 'ok', data: status() });
    mocked.listCalendars.mockResolvedValue({ status: 'ok', data: [workCalendar()] });
    mocked.upcoming.mockResolvedValue({ status: 'ok', data: [] });
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('lists the next meetings under today with their countdown, people and service', async () => {
    const data = [meetingIn('Pricing review', 12), meetingIn('Roadmap sync', 180)];
    mocked.upcoming.mockResolvedValue({ status: 'ok', data });
    renderComingUp();

    const today = await screen.findByRole('region', { name: 'Today' });
    expect(within(today).getByText('Pricing review')).toBeTruthy();
    expect(within(today).getByText('in 12m')).toBeTruthy();
    expect(within(today).getAllByText('2 people · External · Zoom')).toHaveLength(2);
    const range = mocked.upcoming.mock.calls[0]?.[0];
    expect(range && Date.parse(range.end) - Date.parse(range.start)).toBe(15 * 24 * 3600 * 1000);
  });

  it('says so when nothing is coming up', async () => {
    renderComingUp();

    expect(await screen.findByText('Nothing coming up')).toBeTruthy();
  });

  it('offers to set up calendars while sync is off, without asking for events', async () => {
    mocked.status.mockResolvedValue({ status: 'ok', data: status({ syncEnabled: false }) });
    const router = renderComingUp();

    fireEvent.click(await screen.findByRole('button', { name: 'Set up calendars' }));

    expect(await screen.findByText('Settings page')).toBeTruthy();
    expect(router.state.location.search).toEqual({ section: 'calendar' });
    expect(mocked.upcoming).not.toHaveBeenCalled();
  });

  it('switches to the agenda when CalendarChanged reports sync turned on', async () => {
    mocked.status.mockResolvedValue({ status: 'ok', data: status({ syncEnabled: false }) });
    mocked.upcoming.mockResolvedValue({ status: 'ok', data: [meetingIn('Pricing review', 30)] });
    renderComingUp();
    await screen.findByRole('button', { name: 'Set up calendars' });

    const listen = bindings.events.calendarChanged.listen;
    act(() => {
      listen.mock.calls[0]?.[0]({
        event: 'calendar-changed',
        id: 1,
        payload: { status: status() },
      });
    });

    expect(await screen.findByText('Pricing review')).toBeTruthy();
  });
});
