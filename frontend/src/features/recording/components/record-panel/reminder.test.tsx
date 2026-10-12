import { QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { consentCopy } from '@/features/consent';
import { screenCopy } from '@/features/screen';
import { createQueryClient } from '@/lib/ipc';
import type { MeetingId } from '@/lib/ipc/generated/meetings';

import { RecordingReminder } from './reminder';

const consent = vi.hoisted(() => ({ reminder: vi.fn(), stopReminding: vi.fn() }));
const screenCommands = vi.hoisted(() => ({
  status: vi.fn(),
  consentReminder: vi.fn(),
  stopConsentReminders: vi.fn(),
}));
vi.mock('@/lib/ipc/generated/consent', () => ({ commands: consent }));
vi.mock('@/lib/ipc/generated/screen', () => ({ commands: screenCommands }));

const MEETING = 'meeting-1' as MeetingId;

function capturing(on: boolean) {
  screenCommands.status.mockResolvedValue({
    status: 'ok',
    data: {
      supported: true,
      permitted: true,
      capture: {
        meeting: on ? MEETING : null,
        state: on ? 'capturing' : 'off',
        video: true,
        snapshots: 0,
      },
    },
  });
}

function renderReminder() {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <RecordingReminder meetingId={MEETING} />
    </QueryClientProvider>,
  );
}

describe('the recording reminder (UX-28)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    consent.reminder.mockResolvedValue({ status: 'ok', data: true });
    consent.stopReminding.mockResolvedValue({ status: 'ok', data: null });
    screenCommands.consentReminder.mockResolvedValue({ status: 'ok', data: true });
    screenCommands.stopConsentReminders.mockResolvedValue({ status: 'ok', data: null });
  });

  it('shows one card that mentions the shared screen while it is captured', async () => {
    capturing(true);
    renderReminder();

    expect(await screen.findByText(consentCopy.reminder.bodyWithScreen)).toBeTruthy();
    expect(screen.getAllByRole('region')).toHaveLength(1);
    fireEvent.click(screen.getByRole('button', { name: 'Don’t show again' }));

    await waitFor(() => {
      expect(consent.stopReminding).toHaveBeenCalledTimes(1);
      expect(screenCommands.stopConsentReminders).toHaveBeenCalledTimes(1);
    });
    expect(screen.queryByRole('region')).toBeNull();
  });

  it('keeps to the consent words when nothing is captured', async () => {
    capturing(false);
    renderReminder();

    expect(await screen.findByText(consentCopy.reminder.body)).toBeTruthy();
  });

  it('shows the screen card alone when the consent reminder is off', async () => {
    consent.reminder.mockResolvedValue({ status: 'ok', data: false });
    capturing(true);
    renderReminder();

    expect(await screen.findByText(screenCopy.reminder.body)).toBeTruthy();
    expect(screen.queryByText(consentCopy.reminder.body)).toBeNull();
  });
});
