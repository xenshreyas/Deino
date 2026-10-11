import { QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import type { CalendarId } from '@/lib/ipc/generated/calendar';
import { createQueryClient } from '@/lib/ipc';

import { status, workCalendar } from '../test-calendar';
import { CalendarSettings } from './calendar-settings';

const bindings = await vi.hoisted(async () =>
  (await import('../test-calendar')).calendarBindings(),
);
vi.mock('@/lib/ipc/generated/calendar', () => bindings);
const mocked = bindings.commands;

function renderPage() {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <CalendarSettings />
    </QueryClientProvider>,
  );
}

const FAMILY = workCalendar({
  id: 'cal-family' as CalendarId,
  title: 'Family',
  color: '#FF2D55',
});
const HOLIDAYS = workCalendar({
  id: 'cal-holidays' as CalendarId,
  title: 'US Holidays',
  accountId: 'acc-subscribed' as ReturnType<typeof workCalendar>['accountId'],
  accountName: 'Subscribed Calendars',
});

describe('CalendarSettings', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocked.status.mockResolvedValue({
      status: 'ok',
      data: status({ syncEnabled: false, access: 'not_determined', lastSyncedAt: null }),
    });
    mocked.listCalendars.mockResolvedValue({ status: 'ok', data: [] });
  });

  it('explains the macOS prompt before sync is on, and turning it on enables sync', async () => {
    mocked.enable.mockResolvedValue({ status: 'ok', data: status() });
    renderPage();

    expect(
      await screen.findByText('macOS will ask for permission to read your calendars.'),
    ).toBeTruthy();
    fireEvent.click(screen.getByRole('switch', { name: 'Sync calendars' }));

    await waitFor(() => {
      expect(mocked.enable).toHaveBeenCalledOnce();
    });
    expect(await screen.findByText(/^Synced /)).toBeTruthy();
  });

  it('offers System Settings when macOS denied access', async () => {
    const denied = status({ syncEnabled: false, access: 'denied', lastSyncedAt: null });
    mocked.status.mockResolvedValue({ status: 'ok', data: denied });
    mocked.openAccessSettings.mockResolvedValue({ status: 'ok', data: true });
    renderPage();

    fireEvent.click(await screen.findByRole('button', { name: 'Open System Settings' }));

    await waitFor(() => {
      expect(mocked.openAccessSettings).toHaveBeenCalledOnce();
    });
    expect(screen.queryByRole('alert')).toBeNull();
  });

  it('lists calendars by account with their colours, and hides one', async () => {
    mocked.status.mockResolvedValue({ status: 'ok', data: status() });
    mocked.listCalendars.mockResolvedValue({
      status: 'ok',
      data: [FAMILY, workCalendar(), HOLIDAYS],
    });
    mocked.setCalendarVisible.mockResolvedValue({
      status: 'ok',
      data: { ...HOLIDAYS, visible: false },
    });
    renderPage();

    const icloud = await screen.findByRole('group', { name: 'iCloud' });
    expect(within(icloud).getAllByRole('switch')).toHaveLength(2);
    fireEvent.click(screen.getByRole('switch', { name: 'Show US Holidays' }));

    await waitFor(() => {
      expect(mocked.setCalendarVisible).toHaveBeenCalledWith('cal-holidays', false);
    });
    await waitFor(() => {
      const holidays = screen.getByRole('switch', { name: 'Show US Holidays' });
      expect(holidays.getAttribute('aria-checked')).toBe('false');
    });
  });

  it('shows the last sync error and refreshes on demand', async () => {
    const failing = status({ lastError: 'could not read your calendars' });
    mocked.status.mockResolvedValue({ status: 'ok', data: failing });
    mocked.refresh.mockResolvedValue({ status: 'ok', data: status() });
    renderPage();

    expect((await screen.findByRole('alert')).textContent).toBe('could not read your calendars');
    fireEvent.click(screen.getByRole('button', { name: 'Refresh' }));

    await waitFor(() => {
      expect(screen.queryByRole('alert')).toBeNull();
    });
    expect(mocked.refresh).toHaveBeenCalledOnce();
  });
});
