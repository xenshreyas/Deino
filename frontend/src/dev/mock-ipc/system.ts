/** Mock of `@/lib/ipc/generated/system`. */
import type * as Generated from '@/lib/ipc/generated/system';

import packageJson from '../../../package.json';

export const commands = {
  appVersion: () => Promise.resolve(packageJson.version),
} satisfies typeof Generated.commands;
