/**
 * The reminder window's content (ORG-02): the event Rust is reminding of,
 * wired to Join (`calendar|join`, the path Home and the menu bar take) and
 * dismiss. A button closes the window once its call succeeded; the window
 * also closes itself a minute after it showed, as the ticker does.
 *
 * Each event remounts the card (by `key`), which restarts its entrance,
 * its timer and its theme: only the main window's first-paint cache
 * reaches this window.
 */
import { applyTheme, parseThemePreference, THEME_STORAGE_KEY } from '@/features/appearance';
import { useMountEffect } from '@/hooks/useMountEffect';

import { useNow } from '../hooks/use-now';
import { type JoinMode, useJoin } from '../plan';
import { useDismissReminder, useReminder } from '../reminders';
import type { CalendarEvent } from '../types';
import { ReminderCard } from './reminder-card';

/** How long the card stays up unless answered; Rust's `REMINDER_OPEN_FOR`. */
export const REMINDER_OPEN_MS = 60_000;

/** Applies the theme the main window last saved. */
function syncTheme(): void {
  try {
    applyTheme(parseThemePreference(window.localStorage.getItem(THEME_STORAGE_KEY)));
  } catch (error: unknown) {
    console.error('[reminder] Could not read the saved theme', error);
  }
}

/** One event's card, with its buttons and its one-minute timer. */
function EventReminder({ event }: { event: CalendarEvent }) {
  const now = useNow(5_000);
  const join = useJoin();
  const dismiss = useDismissReminder();
  useMountEffect(() => {
    syncTheme();
    const timer = setTimeout(() => {
      dismiss.mutate();
    }, REMINDER_OPEN_MS);
    return () => {
      clearTimeout(timer);
    };
  });
  const onJoin = (mode: JoinMode) => {
    join.mutate(
      { id: event.id, mode },
      {
        onSuccess: () => {
          dismiss.mutate();
        },
      },
    );
  };
  return (
    <ReminderCard
      event={event}
      now={now}
      busy={join.isPending || dismiss.isPending}
      failed={join.isError}
      openForMs={REMINDER_OPEN_MS}
      onJoin={onJoin}
      onDismiss={() => {
        dismiss.mutate();
      }}
    />
  );
}

/** The card for the event being reminded of; nothing between reminders. */
export function MeetingReminder() {
  const reminder = useReminder();
  const event = reminder.data ?? null;
  return event === null ? null : <EventReminder key={event.id} event={event} />;
}
