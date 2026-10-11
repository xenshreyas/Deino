/**
 * The integrations feature (INT-00): Settings → Integrations. It lists the
 * providers Deino can connect (and the planned ones), connects and
 * disconnects accounts through the `integrations` plugin, and shows what
 * was sent. Other code imports from here only.
 */
export { integrationsApi } from './api';
export {
  IntegrationsSettings,
  type IntegrationsSettingsProps,
} from './components/integrations-settings';
export { integrationsCopy } from './copy';
export {
  DELIVERY_PAGE,
  deliveriesQuery,
  integrationKeys,
  providersQuery,
  useDeliveries,
  useProviders,
} from './queries';
export type { Connection, Delivery, IntegrationProvider, ProviderView } from './types';
