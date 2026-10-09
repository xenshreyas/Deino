/**
 * TanStack Query hooks for the network page.
 *
 * The egress log is a table, so `DbChanged` (`egress_log`) refreshes it after
 * every request Deino makes. Service switches live in the settings file, so
 * the list is re-read whenever the page mounts, and a switch writes its
 * result straight into the cache.
 */
import { queryOptions, useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import type { Table } from '@/lib/ipc/generated/shared';

import { networkApi } from './api';
import type { ServiceStatus, ServiceSwitchInput } from './types';

/** How many egress log entries the page shows. */
export const EGRESS_LOG_PAGE = 100;

/** The tables the egress log query reads. */
const EGRESS_TABLES = ['egress_log'] as const satisfies readonly Table[];

/** Query keys; every key starts with `networkKeys.all`. */
export const networkKeys = {
  all: ['network'] as const,
  services: () => [...networkKeys.all, 'services'] as const,
  egressLog: (limit: number) => [...networkKeys.all, 'egress-log', limit] as const,
};

/** Every service; re-read on each mount, since settings can change elsewhere. */
export const servicesQuery = () =>
  queryOptions({ queryKey: networkKeys.services(), queryFn: networkApi.services, staleTime: 0 });

/** The newest outgoing requests, kept fresh by `DbChanged`. */
export const egressLogQuery = (limit: number = EGRESS_LOG_PAGE) =>
  queryOptions({
    queryKey: networkKeys.egressLog(limit),
    queryFn: () => networkApi.egressLog(limit),
    meta: { tables: EGRESS_TABLES },
  });

/** Every service with its hosts and state. */
export function useServices() {
  return useQuery(servicesQuery());
}

/** The newest outgoing requests. */
export function useEgressLog(limit: number = EGRESS_LOG_PAGE) {
  return useQuery(egressLogQuery(limit));
}

/** Switches a service on or off and caches its new status. */
export function useSetServiceEnabled() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: ({ service, enabled }: ServiceSwitchInput) =>
      networkApi.setServiceEnabled(service, enabled),
    onSuccess: (updated) => {
      client.setQueryData(networkKeys.services(), (services: ServiceStatus[] | undefined) =>
        services?.map((service) => (service.id === updated.id ? updated : service)),
      );
    },
  });
}
