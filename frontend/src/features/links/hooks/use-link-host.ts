/**
 * The window's half of `deino://` links and hotkeys (DEV-05): takes the
 * navigations Rust queued and follows them.
 *
 * Mount it inside the app lock's gate (the shell's main layout): it takes
 * on mount, which is right after an unlock, and on each
 * `navigation-waiting` event. Rust refuses to hand anything out while
 * Deino is locked, so a link never shows a meeting behind the lock.
 */
import { useRouter } from '@tanstack/react-router';
import { toast } from 'sonner';

import { calendarApi } from '@/features/calendar';
import { homeApi } from '@/features/home';
import { useMountEffect } from '@/hooks/useMountEffect';
import { useIpcEvent } from '@/lib/ipc';
import { events } from '@/lib/ipc/generated/links';

import { linksApi } from '../api';
import { linksCopy } from '../copy';
import { followAll, type LinkTargets } from '../follow';

/** What the shell supplies. */
export interface LinkHostOptions {
  /** Opens the open meeting's Ask panel; does nothing elsewhere. */
  openAsk: () => void;
}

type Router = ReturnType<typeof useRouter>;

/** The app's actions, over the router and the IPC of other features. */
function targetsFor(router: Router, openAsk: () => void): LinkTargets {
  const openMeeting: LinkTargets['openMeeting'] = (meetingId, atMs) =>
    void router.navigate({
      to: '/meetings/$meetingId',
      params: { meetingId },
      search: atMs === null ? {} : { t: atMs },
    });
  return {
    openMeeting,
    openNewNote: () => void router.navigate({ to: '/' }),
    openEventNote: async (eventId) => {
      const event = await calendarApi.event(eventId);
      openMeeting(await homeApi.openEventNote(event), null);
    },
    openSearch: (query) => void router.navigate({ to: '/search', search: { q: query } }),
    openAsk,
    notify: (message, description) => {
      toast(message, description === undefined ? undefined : { description });
    },
  };
}

/** Takes and follows queued navigations while mounted. */
export function useLinkHost({ openAsk }: LinkHostOptions): void {
  const router = useRouter();
  const drain = () => {
    linksApi
      .take()
      .then((navigations) => followAll(navigations, targetsFor(router, openAsk)))
      .catch((error: unknown) => {
        console.error('[links] could not take the waiting links', error);
      });
  };
  useIpcEvent(events.navigationWaiting, drain);
  useIpcEvent(events.linkRejected, ({ message }) => {
    toast.error(linksCopy.rejected, { description: message });
  });
  useMountEffect(() => {
    drain();
  });
}
