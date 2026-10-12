/**
 * Typed calls to the `api` plugin (the local REST API, DEV-03). Each
 * resolves to the data or throws an `IpcError`; components use the hooks
 * in `queries.ts` instead.
 */
import { commands } from '@/lib/ipc/generated/api';
import { unwrap } from '@/lib/ipc/result';

import type { ApiKey, ApiKeyId, ApiSettings, ApiStatus, CreatedApiKey, NewApiKey } from './types';

export const localApi = {
  /** The switch, the port, and the base URL while the server listens. */
  status: (): Promise<ApiStatus> => unwrap(commands.serverStatus()),
  /** Saves the switch and port; the server starts, stops or moves to match. */
  configure: (settings: ApiSettings): Promise<ApiStatus> =>
    unwrap(commands.configureServer(settings)),
  /** Every key, newest first, revoked ones included. */
  keys: (): Promise<ApiKey[]> => unwrap(commands.listKeys()),
  /** Makes a key; its token is in the reply and never shown again. */
  createKey: (key: NewApiKey): Promise<CreatedApiKey> => unwrap(commands.createKey(key)),
  /** Revokes a key at once. */
  revokeKey: (id: ApiKeyId): Promise<ApiKey> => unwrap(commands.revokeKey(id)),
  /** Removes a key from the list. */
  deleteKey: (id: ApiKeyId): Promise<null> => unwrap(commands.deleteKey(id)),
};
