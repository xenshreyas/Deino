/**
 * The network feature (FND-12): Settings → Privacy → Network. It lists the
 * outside services Deino may talk to, switches them on and off through the
 * `network` plugin, and shows the egress log of every request that left
 * the Mac. Other code imports from here only.
 */
export { networkApi } from './api';
export { NetworkSettings, type NetworkSettingsProps } from './components/network-settings';
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
