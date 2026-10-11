import { useState } from 'react';

import { useMountEffect } from '@/hooks/useMountEffect';

/** How often the agenda's clock ticks: countdowns are in whole minutes. */
export const AGENDA_TICK_MS = 30_000;

/**
 * The current time, re-read every `tickMs` while mounted, so countdowns and
 * "Now" stay true without refetching anything.
 */
export function useNow(tickMs: number = AGENDA_TICK_MS): Date {
  const [now, setNow] = useState(() => new Date());
  useMountEffect(() => {
    const timer = setInterval(() => {
      setNow(new Date());
    }, tickMs);
    return () => {
      clearInterval(timer);
    };
  });
  return now;
}
