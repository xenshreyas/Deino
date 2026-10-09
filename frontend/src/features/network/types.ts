/**
 * The network feature's types, generated from Rust (`deino-net`), never
 * written by hand.
 */
import type { ServiceId } from '@/lib/ipc/generated/network';

export type {
  EgressEntry,
  EgressOutcome,
  Method,
  ServiceId,
  ServiceKind,
  ServiceStatus,
} from '@/lib/ipc/generated/network';

/** Input of the service switch mutation. */
export interface ServiceSwitchInput {
  service: ServiceId;
  enabled: boolean;
}
