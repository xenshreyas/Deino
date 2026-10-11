/**
 * Typed calls to the `developer` plugin (the command-line tool's setup).
 * Each resolves to the data or throws an `IpcError`; components use the
 * hooks in `queries.ts` instead. The debug-only sample seeding is in
 * `sample-api.ts`, so release builds never import the `dev` plugin.
 */
import { commands } from '@/lib/ipc/generated/developer';
import { unwrap } from '@/lib/ipc/result';

import type { CliStatus, InstallDir } from './types';

export const developerApi = {
  /** Where the bundled CLI is, where it is installed, and the Claude snippets. */
  cliStatus: (): Promise<CliStatus> => unwrap(commands.cliStatus()),
  /** Links `deino` in `dir` to the bundled CLI; returns the new status. */
  installCli: (dir: InstallDir): Promise<CliStatus> => unwrap(commands.installCli(dir)),
};
