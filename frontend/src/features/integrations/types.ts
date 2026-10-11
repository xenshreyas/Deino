/**
 * The integrations feature's types, generated from Rust
 * (`deino-integrations`), never written by hand.
 */
import type { IntegrationProvider } from '@/lib/ipc/generated/integrations';

export type {
  Capability,
  ConnectMethod,
  Connection,
  ConnectionId,
  ConnectionStatus,
  Delivery,
  DeliveryStatus,
  IntegrationProvider,
  ProviderView,
  TokenInput,
} from '@/lib/ipc/generated/integrations';

/** Input of the token connect mutation. */
export interface TokenConnectInput {
  provider: IntegrationProvider;
  token: string;
  /** The second field, for providers that pair the token with a username. */
  username: string | null;
}
