/**
 * Typed calls to the `network` plugin. Each resolves to the data or throws
 * an `IpcError`; components use the hooks in `queries.ts` instead.
 */
import { commands } from '@/lib/ipc/generated/network';
import { unwrap } from '@/lib/ipc/result';

import type { EgressEntry, ServiceId, ServiceStatus } from './types';

export const networkApi = {
  /** Every outside service, with its hosts and whether it may send now. */
  services: (): Promise<ServiceStatus[]> => unwrap(commands.services()),
  /** Switches a service on or off; resolves to its new status. */
  setServiceEnabled: (service: ServiceId, enabled: boolean): Promise<ServiceStatus> =>
    unwrap(commands.setServiceEnabled(service, enabled)),
  /** The newest `limit` outgoing requests (Rust caps it at 500), newest first. */
  egressLog: (limit: number): Promise<EgressEntry[]> => unwrap(commands.egressLog(limit)),
};
