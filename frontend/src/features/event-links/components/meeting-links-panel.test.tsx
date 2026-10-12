import { QueryClientProvider } from '@tanstack/react-query';
import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  RouterProvider,
} from '@tanstack/react-router';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, test, vi } from 'vitest';

import { createQueryClient } from '@/lib/ipc';
import { TooltipProvider } from '@/ui/tooltip';

import { calendarEvent } from '../test-event-links';
import type { CalendarEvent, MeetingId } from '../types';
import { MeetingLinksPanel } from './meeting-links-panel';

const bindings = await vi.hoisted(async () => (await import('../test-event-links')).linkBindings());
vi.mock('@/lib/ipc/generated/calendar', () => bindings);

const MEETING = 'meeting-1' as MeetingId;
const design = calendarEvent('Design review', '2026-10-09T10:00:00.000Z');
const pricing = calendarEvent('Pricing review', '2026-10-09T10:30:00.000Z');

function commands() {
  return bindings.commands;
}

function linkedTo(event: CalendarEvent | null) {
  commands().meetingEvent.mockResolvedValue({ status: 'ok', data: event });
}

/** The Details sections at `/`, with meeting pages to open. */
function renderChips() {
  const root = createRootRoute();
  const home = createRoute({
    getParentRoute: () => root,
    path: '/',
    component: () => <MeetingLinksPanel meetingId={MEETING} />,
  });
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
}

describe('the Details panel’s calendar sections', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    linkedTo(design);
    commands().eventSuggestions.mockResolvedValue({ status: 'ok', data: [design, pricing] });
    commands().relatedMeetings.mockResolvedValue({
      status: 'ok',
      data: { previous: null, others: [] },
    });
    commands().linkEvent.mockResolvedValue({
      status: 'ok',
      data: {
        meetingId: MEETING,
        event: pricing,
        title: 'Pricing review',
        attendeesAdded: 1,
        attendeesRemoved: 1,
      },
    });
    commands().unlinkEvent.mockResolvedValue({
      status: 'ok',
      data: {
        meetingId: MEETING,
        event: null,
        title: null,
        attendeesAdded: 0,
        attendeesRemoved: 2,
      },
    });
  });

  test('shows the linked event, and picking another one re-links the meeting', async () => {
    renderChips();

    fireEvent.click(
      await screen.findByRole('button', { name: /Change calendar event: Design review/ }),
    );
    fireEvent.click(await screen.findByText('Pricing review'));

    await waitFor(() => {
      expect(commands().linkEvent).toHaveBeenCalledWith(MEETING, pricing.id);
    });
    expect(await screen.findByRole('button', { name: /Pricing review/ })).toBeDefined();
  });

  test('unlinking turns the chip back into "Link event"', async () => {
    renderChips();

    fireEvent.click(await screen.findByRole('button', { name: /Design review/ }));
    fireEvent.click(await screen.findByText('Unlink event'));

    await waitFor(() => {
      expect(commands().unlinkEvent).toHaveBeenCalledWith(MEETING);
    });
    expect(await screen.findByRole('button', { name: 'Link event' })).toBeDefined();
  });

  test('an unlinked meeting offers "Link event" and the events around it', async () => {
    linkedTo(null);
    renderChips();

    fireEvent.click(await screen.findByRole('button', { name: 'Link event' }));

    expect(await screen.findByText('Around this meeting')).toBeDefined();
    expect(screen.queryByText('Unlink event')).toBeNull();
    expect(commands().eventSuggestions).toHaveBeenCalledWith(MEETING);
  });

  test('shows "Previous in series" when an earlier meeting of the series exists', async () => {
    commands().relatedMeetings.mockResolvedValue({
      status: 'ok',
      data: {
        previous: {
          id: 'meeting-0' as MeetingId,
          title: 'Design review',
          happenedAt: '2026-10-02T10:00:00.000Z',
          by: 'series',
        },
        others: [],
      },
    });
    renderChips();

    const previous = await screen.findByRole('region', { name: 'Previous in series' });
    expect(previous.querySelector('a')?.getAttribute('href')).toBe('/meetings/meeting-0');
    expect(previous.textContent).toContain('Design review');
  });
});
