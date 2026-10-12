/**
 * Typed calls to the `consent` plugin: the reminder's two questions, the
 * only ones the record panel's window may ask. Each resolves to the data or
 * throws an `IpcError`; components use the hooks in `queries.ts`.
 */
import { commands } from '@/lib/ipc/generated/consent';
import { unwrap } from '@/lib/ipc/result';

export const consentApi = {
  /** Whether the reminder card shows when a recording starts. */
  reminder: (): Promise<boolean> => unwrap(commands.reminder()),
  /** "Don't show again": turns the reminder off. */
  stopReminding: (): Promise<null> => unwrap(commands.stopReminding()),
};
