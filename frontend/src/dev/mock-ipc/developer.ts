/**
 * Mock of `@/lib/ipc/generated/developer`: a bundled CLI that is not
 * installed yet; installing into `~/.local/bin` succeeds, `/usr/local/bin`
 * needs an administrator, as on a stock Mac.
 */
import type * as Generated from '@/lib/ipc/generated/developer';

import { failure, ok } from './command-result';
import { latency } from './latency';

type CliStatus = Generated.CliStatus;
type InstallDir = Generated.InstallDir;

const BUNDLED = '/Applications/Deino.app/Contents/MacOS/cli';
const USER_LINK = '/Users/you/.local/bin/deino';
const SYSTEM_LINK = '/usr/local/bin/deino';

/** Whether the mock CLI has been installed into `~/.local/bin`. */
let installed = false;

/** The status the real service would report. */
function status(): CliStatus {
  const command = installed ? USER_LINK : BUNDLED;
  return {
    bundled: BUNDLED,
    targets: [
      {
        dir: 'user_bin',
        path: USER_LINK,
        state: { kind: installed ? 'installed' : 'missing' },
        writable: true,
      },
      { dir: 'usr_local_bin', path: SYSTEM_LINK, state: { kind: 'missing' }, writable: false },
    ],
    command,
    claudeCode: `claude mcp add deino -- ${command} mcp`,
    claudeDesktop: JSON.stringify({ mcpServers: { deino: { command, args: ['mcp'] } } }, null, 2),
  };
}

export const commands = {
  async cliStatus() {
    await latency();
    return ok(status());
  },
  async installCli(dir: InstallDir) {
    await latency();
    if (dir === 'usr_local_bin') {
      return failure<CliStatus>({
        code: 'permission_denied',
        message: `Deino cannot write to that folder; run this in Terminal instead: sudo ln -sf '${BUNDLED}' '${SYSTEM_LINK}'`,
      });
    }
    installed = true;
    return ok(status());
  },
} satisfies typeof Generated.commands;
