/**
 * Typed calls to the `privacy` plugin: the app lock and the library's
 * startup status. Rust's IPC guard always admits them, so they work while
 * the app is locked.
 */
import { commands } from '@/lib/ipc/generated/privacy';
import { unwrap } from '@/lib/ipc/result';

import type { LibraryStatus, LockState } from './types';

export const privacyApi = {
  /** Locked or not, now (an overdue idle lock takes effect here). */
  state: (): Promise<LockState> => unwrap(commands.state()),
  /** The owner used the window: restarts the idle timer, never unlocks. */
  touch: (): Promise<LockState> => unwrap(commands.touch()),
  /** Locks now, when the app lock is on. */
  lock: (): Promise<LockState> => unwrap(commands.lock()),
  /** Shows the Touch ID (or password) prompt and unlocks on success. */
  unlock: (): Promise<LockState> => unwrap(commands.unlock()),
  /** Whether the library is open, being converted, or cannot open. */
  library: (): Promise<LibraryStatus> => unwrap(commands.library()),
};
