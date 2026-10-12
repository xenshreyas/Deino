import { fireEvent, render, screen } from '@testing-library/react';
import { addMinutes } from 'date-fns';
import { describe, expect, it, vi } from 'vitest';

import type { CalendarId } from '@/lib/ipc/generated/calendar';

import { meetingIn } from '../test-calendar';
import { peopleLine, ReminderCard, reminderLead } from './reminder-card';
import { withReminder } from './rules-settings';

const NOW = new Date(2026, 9, 9, 9, 59, 15);

function renderCard(conferenceUrl: string | null = 'https://zoom.us/j/1') {
  vi.useFakeTimers({ toFake: ['Date'], now: NOW });
  const event = { ...meetingIn('Pricing review', 1), conferenceUrl };
  vi.useRealTimers();
  const onJoin = vi.fn();
  const onDismiss = vi.fn();
  render(
    <ReminderCard
      event={event}
      now={NOW}
      busy={false}
      failed={false}
      onJoin={onJoin}
      onDismiss={onDismiss}
    />,
  );
  return { onJoin, onDismiss };
}

describe('ReminderCard', () => {
  it('shows the lead, title and people with both buttons for a call', () => {
    const { onJoin, onDismiss } = renderCard();

    expect(screen.getByRole('region', { name: 'Meeting starting soon' })).toBeTruthy();
    expect(screen.getByText('Starts in a minute')).toBeTruthy();
    expect(screen.getByRole('heading', { name: 'Pricing review' })).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: 'Join & record' }));
    fireEvent.click(screen.getByRole('button', { name: 'Record only' }));
    fireEvent.click(screen.getByRole('button', { name: 'Dismiss' }));
    expect(onJoin.mock.calls).toEqual([['joinAndRecord'], ['recordOnly']]);
    expect(onDismiss).toHaveBeenCalledOnce();
  });

  it('offers only Record only without a call link', () => {
    renderCard(null);

    expect(screen.queryByRole('button', { name: 'Join & record' })).toBeNull();
    expect(screen.getByRole('button', { name: 'Record only' })).toBeTruthy();
  });
});

describe('reminder helpers', () => {
  it('says "Starting now" once the meeting has started', () => {
    const event = meetingIn('Standup', 0);
    expect(reminderLead(event, addMinutes(new Date(event.startAt), 1))).toBe('Starting now');
    expect(reminderLead(event, addMinutes(new Date(event.startAt), -3))).toBe(
      'Starts in 3 minutes',
    );
  });

  it('names three people, then counts the rest', () => {
    const event = meetingIn('All hands', 1);
    const person = (name: string) => ({ email: null, name, rsvp: null, isOrganizer: false });
    event.attendees = ['Ada', 'Bo', 'Cy', 'Di', 'Ed'].map(person);
    expect(peopleLine(event)).toBe('Ada, Bo, Cy +2');
  });

  it('adds a calendar to the quiet list when its reminders go off, and back', () => {
    const work = 'cal-work' as CalendarId;
    const home = 'cal-home' as CalendarId;
    expect(withReminder([home], work, false)).toEqual([home, work]);
    expect(withReminder([home, work], work, true)).toEqual([home]);
  });
});
