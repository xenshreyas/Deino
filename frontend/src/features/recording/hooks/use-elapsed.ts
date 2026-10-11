/** The recording timer, re-rendering once a second while mounted. */
import { useState } from 'react';

import { useMountEffect } from '@/hooks/useMountEffect';

import { elapsedSeconds } from '../elapsed';
import type { RecordingSnapshot } from '../types';

const TICK_MS = 1000;

/** Seconds recorded so far by `snapshot`'s recording. */
export function useElapsed(snapshot: RecordingSnapshot): number {
  const [now, setNow] = useState(Date.now);
  useMountEffect(() => {
    const timer = window.setInterval(() => {
      setNow(Date.now());
    }, TICK_MS);
    return () => {
      window.clearInterval(timer);
    };
  });
  return elapsedSeconds(snapshot, now);
}
