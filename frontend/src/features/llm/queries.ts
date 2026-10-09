/**
 * TanStack Query hooks for Settings → AI.
 *
 * Providers and routes depend on saved keys and settings, which live in the
 * Keychain and the settings file rather than tables, so `DbChanged` never
 * covers them: every `SettingsChanged` invalidates them instead
 * ({@link useLlmSettingsChanges}), and a route change writes its result
 * straight into the cache.
 */
import {
  queryOptions,
  type QueryClient,
  skipToken,
  useMutation,
  useQuery,
  useQueryClient,
} from '@tanstack/react-query';
import { useState } from 'react';

import { useIpcEvent } from '@/lib/ipc/events';
import { events as settingsEvents } from '@/lib/ipc/generated/settings';

import { llmApi } from './api';
import type { ProviderId, ResolvedRoute, Route, Target } from './types';

/** Query keys; every key starts with `llmKeys.all`. */
export const llmKeys = {
  all: ['llm'] as const,
  providers: () => [...llmKeys.all, 'providers'] as const,
  routes: () => [...llmKeys.all, 'routes'] as const,
  models: (provider: ProviderId | null) => [...llmKeys.all, 'models', provider] as const,
};

/** Model lists change rarely and cost a network call: keep them five minutes. */
const MODELS_STALE_TIME = 5 * 60 * 1000;

/** Every provider with its masked key. */
export const providersQuery = () =>
  queryOptions({ queryKey: llmKeys.providers(), queryFn: llmApi.providers });

/** Every route and what serves it. */
export const routesQuery = () =>
  queryOptions({ queryKey: llmKeys.routes(), queryFn: llmApi.routes });

/** Every provider with its masked key. */
export function useProviders() {
  return useQuery(providersQuery());
}

/** Every route and what serves it. */
export function useRoutes() {
  return useQuery(routesQuery());
}

/** The models of `provider`, with the saved key and endpoint. */
export const modelsQuery = (provider: ProviderId) =>
  queryOptions({
    queryKey: llmKeys.models(provider),
    queryFn: () => llmApi.models(provider),
    staleTime: MODELS_STALE_TIME,
  });

/** The models of `provider`; idle while `provider` is `null`. */
export function useModels(provider: ProviderId | null) {
  return useQuery({
    queryKey: llmKeys.models(provider),
    queryFn: provider === null ? skipToken : () => llmApi.models(provider),
    staleTime: MODELS_STALE_TIME,
  });
}

/** Input of {@link useSetRoute}. */
export interface SetRouteInput {
  route: Route;
  target: Target | null;
}

/** Pins or resets a route and caches the result. */
export function useSetRoute() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: ({ route, target }: SetRouteInput) => llmApi.setRoute(route, target),
    onSuccess: (resolved) => {
      cacheRoute(client, resolved);
    },
  });
}

/**
 * Changing one route: a model of its provider, a new provider (starting on
 * the first model it lists), or back to its default. Every handler returns
 * at once; `error` holds the last failure.
 */
export function useRoutePin(row: ResolvedRoute) {
  const client = useQueryClient();
  const setRoute = useSetRoute();
  const pin = (target: Target | null) => {
    setRoute.mutate({ route: row.route, target });
  };
  const chooseProvider = (provider: ProviderId) => {
    void client
      .query(modelsQuery(provider))
      .catch(() => [])
      .then((models) => {
        pin({ provider, model: models[0]?.id ?? row.target.model });
      });
  };
  return {
    reset: () => {
      pin(null);
    },
    chooseProvider,
    chooseModel: (model: string) => {
      pin({ provider: row.target.provider, model });
    },
    error: setRoute.error,
  };
}

/**
 * A route test: the streamed reply so far, and the report when it ends.
 * Starting a new test clears the previous reply.
 */
export function useTestRoute(route: Route) {
  const [reply, setReply] = useState('');
  const mutation = useMutation({
    mutationFn: () => {
      setReply('');
      return llmApi.testRoute(route, (delta) => {
        setReply((previous) => previous + delta);
      });
    },
  });
  return { ...mutation, reply };
}

/** Refetches providers and routes after every saved settings change. Mount once. */
export function useLlmSettingsChanges(): void {
  const client = useQueryClient();
  useIpcEvent(settingsEvents.settingsChanged, () => {
    void client.invalidateQueries({ queryKey: llmKeys.all });
  });
}

function cacheRoute(client: QueryClient, resolved: ResolvedRoute): void {
  client.setQueryData<ResolvedRoute[]>(llmKeys.routes(), (routes) =>
    routes?.map((row) => (row.route === resolved.route ? resolved : row)),
  );
}
