/**
 * TanStack Query hooks for webhooks.
 *
 * Queries name the tables they read, so `DbChanged` refreshes them after
 * any write, including the dispatcher's: the log shows each attempt as it
 * lands and an endpoint turned off for failing shows its notice at once.
 */
import { queryOptions, useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import type { Table } from '@/lib/ipc/generated/shared';

import { webhooksApi } from './api';
import type { WebhookId, WebhookInput } from './types';

/** The endpoints. */
const WEBHOOK_TABLES = ['webhooks'] as const satisfies readonly Table[];
/** The delivery log. */
const DELIVERY_TABLES = ['webhooks', 'webhook_deliveries'] as const satisfies readonly Table[];

/** Query keys; every key starts with `webhookKeys.all`. */
export const webhookKeys = {
  all: ['webhooks'] as const,
  list: () => [...webhookKeys.all, 'list'] as const,
  deliveries: (id: WebhookId) => [...webhookKeys.all, 'deliveries', id] as const,
};

/** Every endpoint. Also usable from route loaders. */
export const webhookListQuery = () =>
  queryOptions({
    queryKey: webhookKeys.list(),
    queryFn: webhooksApi.list,
    meta: { tables: WEBHOOK_TABLES },
  });

/** Every endpoint, oldest first. */
export function useWebhooks() {
  return useQuery(webhookListQuery());
}

/** An endpoint's newest deliveries. */
export function useDeliveries(id: WebhookId) {
  return useQuery({
    queryKey: webhookKeys.deliveries(id),
    queryFn: () => webhooksApi.deliveries(id),
    meta: { tables: DELIVERY_TABLES },
  });
}

/** A mutation that refreshes every webhook query when it settles. */
function useWebhookMutation<TArgs, TResult>(run: (args: TArgs) => Promise<TResult>) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: run,
    onSettled: () => queryClient.invalidateQueries({ queryKey: webhookKeys.all }),
  });
}

/** Adds an endpoint; its data carries the secret, shown once. */
export function useCreateWebhook() {
  return useWebhookMutation(webhooksApi.create);
}

/** Changes an endpoint. */
export function useUpdateWebhook() {
  return useWebhookMutation(({ id, input }: { id: WebhookId; input: WebhookInput }) =>
    webhooksApi.update(id, input),
  );
}

/** Removes an endpoint. */
export function useDeleteWebhook() {
  return useWebhookMutation(webhooksApi.delete);
}

/** Replaces an endpoint's secret; its data is the new one, shown once. */
export function useRotateSecret() {
  return useWebhookMutation(webhooksApi.rotateSecret);
}

/** Sends the latest meeting to an endpoint now. */
export function useSendTest() {
  return useWebhookMutation(webhooksApi.sendTest);
}
