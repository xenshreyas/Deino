/**
 * The network feature (FND-12): Settings → Privacy → Network (offline mode
 * and the egress log of every request that left the Mac) and Settings →
 * Advanced → Network services (the outside services Deino may talk to,
 * switched on and off through the `network` plugin). Other code imports
 * from here only.
 */
export { networkApi } from './api';
export { NetworkSettings, type NetworkSettingsProps } from './components/network-settings';
export { ServicesSection as NetworkServices } from './components/service-list';
export { networkCopy } from './copy';
export {
  EGRESS_LOG_PAGE,
  egressLogQuery,
  networkKeys,
  servicesQuery,
  useEgressLog,
  useServices,
  useSetServiceEnabled,
} from './queries';
export type { EgressEntry, ServiceId, ServiceStatus } from './types';
