/**
 * Mock of `@/lib/ipc/generated/privacy`: an app lock in memory that follows
 * `privacy.appLock`, and the library status.
 *
 * `unlock` stands in for Touch ID and always succeeds. The library opens
 * ready (encrypted when the settings say so) unless the URL asks for
 * another startup screen: `?library=encrypting`, `?library=decrypting` or
 * `?library=key_missing` (and the other `LibraryProblem` values).
 */
import type * as Generated from '@/lib/ipc/generated/privacy';

import { ok } from './command-result';
import { latency } from './latency';
import { currentSettings } from './settings-store';

let lockState: Generated.LockState = 'unlocked';

/** Problems `?library=` may name. */
const PROBLEMS: readonly Generated.LibraryProblem[] = [
  'key_missing',
  'key_denied',
  'wrong_key',
  'open_failed',
];

/** The startup status the URL asks for, or a ready library. */
function libraryStatus(): Generated.LibraryStatus {
  const asked = new URLSearchParams(window.location.search).get('library');
  if (asked === 'encrypting') return { kind: 'converting', direction: 'encrypt' };
  if (asked === 'decrypting') return { kind: 'converting', direction: 'decrypt' };
  const problem = PROBLEMS.find((candidate) => candidate === asked);
  if (problem !== undefined) return { kind: 'unavailable', problem };
  const encrypted = currentSettings().privacy.encryptDatabase;
  return { kind: 'ready', encrypted, outcome: { kind: 'unchanged' } };
}

/** The lock is only ever on when the settings turn it on. */
function lockEnabled(): boolean {
  return currentSettings().privacy.appLock.enabled;
}

export const commands = {
  async state() {
    await latency();
    if (!lockEnabled()) lockState = 'unlocked';
    return ok(lockState);
  },
  async touch() {
    await latency();
    return ok(lockState);
  },
  async lock() {
    await latency();
    if (lockEnabled()) lockState = 'locked';
    return ok(lockState);
  },
  async unlock() {
    await latency();
    lockState = 'unlocked';
    return ok(lockState);
  },
  async library() {
    await latency();
    return ok(libraryStatus());
  },
} satisfies typeof Generated.commands;
