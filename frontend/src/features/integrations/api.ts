/**
 * Typed calls to the `integrations` plugin. Each resolves to the data or
 * throws an `IpcError`; components use the hooks in `queries.ts` instead.
 */
import { commands } from '@/lib/ipc/generated/integrations';
import { unwrap } from '@/lib/ipc/result';

import type {
  Connection,
  ConnectionId,
  Delivery,
  IntegrationProvider,
  ProviderView,
  TokenConnectInput,
} from './types';

export const integrationsApi = {
  /** Every shipped provider with its connections, then the planned ones. */
  providers: (): Promise<ProviderView[]> => unwrap(commands.providers()),
  /** Signs in to `provider` in the browser; resolves when the user is done. */
  connect: (provider: IntegrationProvider): Promise<Connection> =>
    unwrap(commands.connect(provider)),
  /** Stops waiting for the browser; false when nothing was waiting. */
  cancelConnect: (provider: IntegrationProvider): Promise<boolean> =>
    unwrap(commands.cancelConnect(provider)),
  /** Connects with a pasted token, which goes straight to the Keychain. */
  connectToken: ({ provider, token, username }: TokenConnectInput): Promise<Connection> =>
    unwrap(commands.connectToken(provider, { token, username })),
  /** Asks the provider whether the connection still works. */
  check: (connection: ConnectionId): Promise<Connection> => unwrap(commands.check(connection)),
  /** Revokes, forgets and removes a connection. */
  disconnect: (connection: ConnectionId): Promise<null> => unwrap(commands.disconnect(connection)),
  /** The newest `limit` deliveries (Rust caps it at 200). */
  deliveries: (limit: number): Promise<Delivery[]> => unwrap(commands.deliveries(limit)),
};
