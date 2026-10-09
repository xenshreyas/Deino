/**
 * The auto-start countdown as the prompt shows it. Rust owns the real
 * deadline (`autoStartAt`) and starts recording itself; the prompt only
 * counts down to the same instant.
 */
import { useState } from 'react';

import { useMountEffect } from '@/hooks/useMountEffect';

/** How often the countdown re-reads the clock, in milliseconds. */
const TICK_MS = 250;

/** Whole seconds from `now` (epoch ms) until the ISO `deadline`, never negative. */
export function secondsLeft(deadline: string, now: number): number {
  return Math.max(0, Math.ceil((Date.parse(deadline) - now) / 1000));
}

/** Milliseconds from `now` until the ISO `deadline`, never negative. */
export function millisecondsLeft(deadline: string, now: number): number {
  return Math.max(0, Date.parse(deadline) - now);
}

/** The current time, refreshed every {@link TICK_MS} while mounted. */
export function useNow(): number {
  const [now, setNow] = useState(() => Date.now());
  useMountEffect(() => {
    const timer = window.setInterval(() => {
      setNow(Date.now());
    }, TICK_MS);
    return () => {
      window.clearInterval(timer);
    };
  });
  return now;
}
