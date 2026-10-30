import { QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen } from '@testing-library/react';
import type { ReactNode } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import type { MeetingId } from '@/lib/ipc/generated/meetings';
import { createQueryClient } from '@/lib/ipc';
import { TooltipProvider } from '@/ui/tooltip';

import type { FollowUpDraft } from '../types';
import { FollowUpBlock, MeetingFollowUp } from './follow-up-block';

const commands = vi.hoisted(() => ({
  suggested: vi.fn(),
  forMeeting: vi.fn(),
  save: vi.fn(),
  dismiss: vi.fn(),
  openInMail: vi.fn(),
  copy: vi.fn(),
}));
vi.mock('@/lib/ipc/generated/followups', () => ({ commands }));

const MEETING = 'meeting-1' as MeetingId;

const draft: FollowUpDraft = {
  meetingId: MEETING,
  subject: 'Pilot next steps',
  body: 'Hi Marcus,\n\nThe proposal follows by [date].',
  recipients: { to: ['marcus@brightkite.example'], cc: [], bcc: [] },
  generatedAt: '2026-09-30T18:00:00Z',
  editedAt: null,
};

function wrap(children: ReactNode) {
  return (
    <QueryClientProvider client={createQueryClient()}>
      <TooltipProvider>{children}</TooltipProvider>
    </QueryClientProvider>
  );
}

describe('FollowUpBlock', () => {
  it('shows the subject and recipients, opens the draft, or dismisses it for good', () => {
    const onOpen = vi.fn();
    const onDismiss = vi.fn();
    const twoPeople: FollowUpDraft = {
      ...draft,
      recipients: {
        to: ['marcus@brightkite.example'],
        cc: ['MARCUS@brightkite.example', 'hannah@brightkite.example'],
        bcc: [],
      },
    };
    render(wrap(<FollowUpBlock draft={twoPeople} onOpen={onOpen} onDismiss={onDismiss} />));

    screen.getByRole('region', { name: 'Follow-up email' });
    screen.getByText('Pilot next steps');
    screen.getByText('Draft ready · to 2 people');
    fireEvent.click(screen.getByRole('button', { name: 'Open draft' }));
    expect(onOpen).toHaveBeenCalledOnce();
    expect(onDismiss).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole('button', { name: 'Dismiss for good' }));
    expect(onDismiss).toHaveBeenCalledOnce();
  });
});

describe('MeetingFollowUp', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    commands.openInMail.mockResolvedValue({ status: 'ok', data: null });
  });

  it('shows nothing without a draft', async () => {
    commands.forMeeting.mockResolvedValue({ status: 'ok', data: null });
    const { container } = render(wrap(<MeetingFollowUp meetingId={MEETING} />));

    await vi.waitFor(() => {
      expect(commands.forMeeting).toHaveBeenCalledWith(MEETING);
    });
    expect(container.innerHTML).toBe('');
  });

  it('opens the editor pre-filled and sends the edit to Mail', async () => {
    commands.forMeeting.mockResolvedValue({ status: 'ok', data: draft });
    render(wrap(<MeetingFollowUp meetingId={MEETING} />));

    fireEvent.click(await screen.findByRole('button', { name: 'Open draft' }));
    expect(screen.getByLabelText<HTMLInputElement>('To').value).toBe('marcus@brightkite.example');
    screen.getByText('1 gap to fill');
    expect(screen.queryByRole('button', { name: /Send via Gmail or Outlook/ })).toBeNull();
    fireEvent.change(screen.getByLabelText('Message'), { target: { value: 'Hi Marcus.' } });
    fireEvent.click(screen.getByRole('button', { name: 'Open in Mail' }));

    await vi.waitFor(() => {
      expect(commands.openInMail).toHaveBeenCalledWith(MEETING, {
        subject: 'Pilot next steps',
        body: 'Hi Marcus.',
        to: ['marcus@brightkite.example'],
        cc: [],
        bcc: [],
      });
    });
    screen.getByText('No gaps left');
  });
});
