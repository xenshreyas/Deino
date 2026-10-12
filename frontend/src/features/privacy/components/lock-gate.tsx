/**
 * Renders the app, or the lock screen while Deino is locked (PRV-02). The
 * app is unmounted while locked, so nothing behind the lock refetches (the
 * Rust guard would refuse it anyway) and nothing stays on screen.
 */
import type { ReactNode } from 'react';

import { useActivityPings } from '../hooks/use-activity-pings';
import { useLockActions, useLockState } from '../queries';
import { LockScreen } from './lock-screen';

/** The app behind the lock. */
export function LockGate({ children }: { children: ReactNode }) {
  useActivityPings();
  const state = useLockState();
  const { unlock } = useLockActions();
  if (state === 'unlocked') return children;
  return (
    <LockScreen
      unlocking={unlock.isPending}
      onUnlock={() => {
        unlock.mutate();
      }}
    />
  );
}
