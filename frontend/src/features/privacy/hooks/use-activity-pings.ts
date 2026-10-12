/**
 * Tells Rust the owner is using the window, so the idle lock restarts its
 * timer. Pings are throttled: the idle time is counted in minutes.
 */
import { useQueryClient } from '@tanstack/react-query';

import { useMountEffect } from '@/hooks/useMountEffect';

import { privacyApi } from '../api';
import { privacyKeys } from '../queries';

/** The fewest milliseconds between two activity pings. */
export const ACTIVITY_THROTTLE_MS = 15_000;

/** What counts as using the window. */
const ACTIVITY_EVENTS = ['pointerdown', 'keydown', 'wheel'] as const;

/** Pings `touch` on input, at most every {@link ACTIVITY_THROTTLE_MS}. */
export function useActivityPings(now: () => number = Date.now): void {
  const queryClient = useQueryClient();
  useMountEffect(() => {
    let last = Number.NEGATIVE_INFINITY;
    const ping = () => {
      const time = now();
      if (time - last < ACTIVITY_THROTTLE_MS) return;
      last = time;
      void privacyApi.touch().then(
        (state) => queryClient.setQueryData(privacyKeys.lock(), state),
        () => undefined,
      );
    };
    for (const name of ACTIVITY_EVENTS) window.addEventListener(name, ping, { passive: true });
    return () => {
      for (const name of ACTIVITY_EVENTS) window.removeEventListener(name, ping);
    };
  });
}
