/**
 * Typed calls to the `example` plugin.
 *
 * The only place this feature touches IPC: each function calls one generated
 * command and `unwrap`s its result, so callers get data or an `IpcError`.
 */
import { commands } from '@/lib/ipc/generated/example';
import { unwrap } from '@/lib/ipc/result';

import type { ExampleNote } from './types';

export const exampleApi = {
  /** Every note, newest first. */
  list: (): Promise<ExampleNote[]> => unwrap(commands.list()),
  /** Saves a note; rejects with `invalid` for blank or overlong text. */
  add: (text: string): Promise<ExampleNote> => unwrap(commands.add(text)),
};
