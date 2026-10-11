/**
 * TanStack Query hooks for Settings → Integrations.
 *
 * Connections and deliveries are tables, so `DbChanged` refreshes both lists
 * after every connect, check, delivery and disconnect, wherever it came from
 * (an automation, a share). Mutations only report their own outcome.
 */
import { queryOptions, useMutation, useQuery } from '@tanstack/react-query';

import type { Table } from '@/lib/ipc/generated/shared';

import { integrationsApi } from './api';

/** How many deliveries the page shows. */
export const DELIVERY_PAGE = 20;

/** The tables the provider list reads. */
const PROVIDER_TABLES = [
  'integration_connections',
  'integration_connection_scopes',
] as const satisfies readonly Table[];

/** The tables the delivery log reads. */
const DELIVERY_TABLES = ['integration_deliveries'] as const satisfies readonly Table[];

/** Query keys; every key starts with `integrationKeys.all`. */
export const integrationKeys = {
  all: ['integrations'] as const,
  providers: () => [...integrationKeys.all, 'providers'] as const,
  deliveries: (limit: number) => [...integrationKeys.all, 'deliveries', limit] as const,
};

/** Every provider with its connections, kept fresh by `DbChanged`. */
export const providersQuery = () =>
  queryOptions({
    queryKey: integrationKeys.providers(),
    queryFn: integrationsApi.providers,
    meta: { tables: PROVIDER_TABLES },
  });

/** The newest deliveries, kept fresh by `DbChanged`. */
export const deliveriesQuery = (limit: number = DELIVERY_PAGE) =>
  queryOptions({
    queryKey: integrationKeys.deliveries(limit),
    queryFn: () => integrationsApi.deliveries(limit),
    meta: { tables: DELIVERY_TABLES },
  });

/** Every provider with its connections. */
export function useProviders() {
  return useQuery(providersQuery());
}

/** The newest deliveries. */
export function useDeliveries(limit: number = DELIVERY_PAGE) {
  return useQuery(deliveriesQuery(limit));
}

/** Connects with a pasted token. */
export function useConnectToken() {
  return useMutation({ mutationFn: integrationsApi.connectToken });
}

/** Signs in in the browser. */
export function useConnect() {
  return useMutation({ mutationFn: integrationsApi.connect });
}

/** Stops waiting for the browser. */
export function useCancelConnect() {
  return useMutation({ mutationFn: integrationsApi.cancelConnect });
}

/** Asks the provider whether a connection still works. */
export function useCheckConnection() {
  return useMutation({ mutationFn: integrationsApi.check });
}

/** Removes a connection. */
export function useDisconnect() {
  return useMutation({ mutationFn: integrationsApi.disconnect });
}
