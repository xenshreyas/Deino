import { QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { createQueryClient } from '@/lib/ipc';

import { ConsentReminder, shouldRemind } from './consent-reminder';

const bindings = vi.hoisted(() => ({
  commands: {
    reminder: vi.fn(),
    stopReminding: vi.fn(),
  },
}));
vi.mock('@/lib/ipc/generated/consent', () => bindings);
const mocked = bindings.commands;

function renderReminder(meetingId: string | null) {
  const client = createQueryClient();
  const view = render(
    <QueryClientProvider client={client}>
      <ConsentReminder meetingId={meetingId} />
    </QueryClientProvider>,
  );
  return {
    rerender: (next: string | null) => {
      view.rerender(
        <QueryClientProvider client={client}>
          <ConsentReminder meetingId={next} />
        </QueryClientProvider>,
      );
    },
  };
}

describe('shouldRemind', () => {
  it('reminds once per recorded meeting while the setting is on', () => {
    expect(shouldRemind(true, 'm1', null)).toBe(true);
    expect(shouldRemind(true, 'm1', 'm1')).toBe(false);
    expect(shouldRemind(true, 'm2', 'm1')).toBe(true);
    expect(shouldRemind(false, 'm1', null)).toBe(false);
    expect(shouldRemind(undefined, 'm1', null)).toBe(false);
    expect(shouldRemind(true, null, null)).toBe(false);
  });
});

describe('ConsentReminder', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocked.reminder.mockResolvedValue({ status: 'ok', data: true });
    mocked.stopReminding.mockResolvedValue({ status: 'ok', data: null });
  });

  it('shows when a recording starts and hides for this meeting on "Got it"', async () => {
    const view = renderReminder('m1');

    fireEvent.click(await screen.findByRole('button', { name: 'Got it' }));

    expect(screen.queryByRole('region', { name: 'Consent reminder' })).toBeNull();
    expect(mocked.stopReminding).not.toHaveBeenCalled();
    view.rerender('m2');
    expect(await screen.findByRole('region', { name: 'Consent reminder' })).toBeTruthy();
  });

  it('turns the reminder off on "Don’t show again"', async () => {
    renderReminder('m1');

    fireEvent.click(await screen.findByRole('button', { name: 'Don’t show again' }));

    await waitFor(() => {
      expect(mocked.stopReminding).toHaveBeenCalledTimes(1);
    });
    expect(screen.queryByRole('region', { name: 'Consent reminder' })).toBeNull();
  });

  it('stays hidden when the setting is off or nothing records', async () => {
    mocked.reminder.mockResolvedValue({ status: 'ok', data: false });
    const view = renderReminder('m1');
    await waitFor(() => {
      expect(mocked.reminder).toHaveBeenCalled();
    });
    expect(screen.queryByRole('region')).toBeNull();

    view.rerender(null);
    expect(screen.queryByRole('region')).toBeNull();
  });
});
