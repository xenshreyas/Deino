/**
 * What each navigation from a `deino://` link or hotkey does in the window
 * (DEV-05). Rust has parsed and validated the link already; this only maps
 * the typed {@link Navigation} onto the app's own actions, which the link
 * host passes in, so the mapping is tested without a router.
 */
import { linksCopy } from './copy';
import type { CalendarEventId, MeetingId, Navigation } from './types';

/** The app's actions a navigation can lead to. */
export interface LinkTargets {
  /** The meeting's page, on the transcript at `atMs` when given. */
  openMeeting: (meetingId: MeetingId, atMs: number | null) => void;
  /** The new-note screen (what ⌘N opens). */
  openNewNote: () => void;
  /** The calendar event's note, created and linked when it is new. */
  openEventNote: (eventId: CalendarEventId) => Promise<void>;
  /** The search page for `query`. */
  openSearch: (query: string) => void;
  /** The open meeting's Ask panel, when a meeting is open. */
  openAsk: () => void;
  /** A short notice. */
  notify: (message: string, description?: string) => void;
}

/** Does what `navigation` says. */
export async function follow(navigation: Navigation, targets: LinkTargets): Promise<void> {
  switch (navigation.kind) {
    case 'meeting':
      targets.openMeeting(navigation.meetingId, navigation.atMs);
      return;
    case 'new_note':
      if (navigation.eventId === null) targets.openNewNote();
      else await openEventNote(navigation.eventId, targets);
      return;
    case 'search':
      targets.openSearch(navigation.query);
      return;
    case 'recipe':
      // Recipes is not built yet (D12-style WIP): say so instead of failing.
      targets.notify(linksCopy.recipeWip, linksCopy.recipeWipDescription);
      return;
    case 'ask':
      targets.openAsk();
      return;
  }
}

async function openEventNote(eventId: CalendarEventId, targets: LinkTargets): Promise<void> {
  try {
    await targets.openEventNote(eventId);
  } catch {
    targets.notify(linksCopy.eventNoteFailed);
    targets.openNewNote();
  }
}

/** Follows each navigation in order; one that fails never stops the rest. */
export async function followAll(
  navigations: readonly Navigation[],
  targets: LinkTargets,
): Promise<void> {
  for (const navigation of navigations) {
    await follow(navigation, targets);
  }
}
