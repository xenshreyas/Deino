import { fireEvent, render, screen, waitFor } from '@testing-library/react';
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
      openForMs={60_000}
      onJoin={onJoin}
      onDismiss={onDismiss}
    />,
  );
  return { onJoin, onDismiss };
}

/** Opens the caret menu beside the main button (Radix opens on pointer down). */
function openMore() {
  fireEvent.pointerDown(screen.getByRole('button', { name: 'More options' }), {
    button: 0,
    ctrlKey: false,
  });
}

describe('ReminderCard', () => {
  it('joins the call and opens Deino from one button', async () => {
    const { onJoin } = renderCard();

    expect(screen.getByRole('region', { name: 'Meeting starting soon' })).toBeTruthy();
    expect(screen.getByRole('heading', { name: 'Pricing review' })).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: /Join meeting/ }));
    // The card slides out first, then answers.
    await waitFor(() => {
      expect(onJoin).toHaveBeenCalledWith('joinAndRecord');
    });
  });

  it('takes notes without joining, or dismisses, from the caret menu', async () => {
    const { onJoin } = renderCard();

    openMore();
    fireEvent.click(await screen.findByRole('menuitem', { name: 'Take notes without joining' }));
    await waitFor(() => {
      expect(onJoin).toHaveBeenCalledWith('recordOnly');
    });
  });

  it('just takes notes without a call link', async () => {
    const { onJoin, onDismiss } = renderCard(null);

    expect(screen.queryByRole('button', { name: /Join meeting/ })).toBeNull();
    openMore();
    expect(screen.queryByRole('menuitem', { name: 'Take notes without joining' })).toBeNull();
    fireEvent.click(await screen.findByRole('menuitem', { name: 'Dismiss' }));
    await waitFor(() => {
      expect(onDismiss).toHaveBeenCalledOnce();
    });
    expect(onJoin).not.toHaveBeenCalled();
  });

  it('says Take notes on the button without a call link', () => {
    renderCard(null);

    expect(screen.getByRole('button', { name: 'Take notes' })).toBeTruthy();
  });
});

describe('reminder helpers', () => {
  it('says "Starting now" once the meeting has started', () => {
    const event = meetingIn('Standup', 0);
    expect(reminderLead(event, addMinutes(new Date(event.startAt), 1))).toBe('Starting now');
    expect(reminderLead(event, addMinutes(new Date(event.startAt), -3))).toBe('In 3 min');
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
