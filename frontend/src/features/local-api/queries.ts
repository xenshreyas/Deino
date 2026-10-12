/**
 * TanStack Query hooks for Settings → Developer's local API.
 *
 * Keys live in the `api_keys` table, so `DbChanged` keeps the list fresh
 * (a request through the API updates "last used"). The server's state is
 * not in the database: a change writes its result straight into the cache.
 */
import { queryOptions, useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import { localApi } from './api';
import type { ApiKeyId, ApiSettings, NewApiKey } from './types';

/** Query keys; every key starts with `localApiKeys.all`. */
export const localApiKeys = {
  all: ['local-api'] as const,
  status: () => [...localApiKeys.all, 'status'] as const,
  keys: () => [...localApiKeys.all, 'keys'] as const,
};

/** The server's switch, port and address; re-read on each mount. */
export const apiStatusQuery = () =>
  queryOptions({ queryKey: localApiKeys.status(), queryFn: localApi.status, staleTime: 0 });

/** Every API key. */
export const apiKeysQuery = () =>
  queryOptions({
    queryKey: localApiKeys.keys(),
    queryFn: localApi.keys,
    meta: { tables: ['api_keys'] },
  });

/** The server's state. */
export function useApiStatus() {
  return useQuery(apiStatusQuery());
}

/** The keys. */
export function useApiKeys() {
  return useQuery(apiKeysQuery());
}

/** Turns the server on or off, or moves it to another port. */
export function useConfigureApi() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (settings: ApiSettings) => localApi.configure(settings),
    onSuccess: (status) => {
      queryClient.setQueryData(localApiKeys.status(), status);
    },
  });
}

/** Re-reads the keys after a change (`DbChanged` does too, in the app). */
function useRefreshKeys() {
  const queryClient = useQueryClient();
  return () => queryClient.invalidateQueries({ queryKey: localApiKeys.keys() });
}

/** Makes a key; the mutation's data holds the one-time token. */
export function useCreateApiKey() {
  const onSuccess = useRefreshKeys();
  return useMutation({ mutationFn: (key: NewApiKey) => localApi.createKey(key), onSuccess });
}

/** Revokes a key. */
export function useRevokeApiKey() {
  const onSuccess = useRefreshKeys();
  return useMutation({ mutationFn: (id: ApiKeyId) => localApi.revokeKey(id), onSuccess });
}

/** Removes a key from the list. */
export function useDeleteApiKey() {
  const onSuccess = useRefreshKeys();
  return useMutation({ mutationFn: (id: ApiKeyId) => localApi.deleteKey(id), onSuccess });
}
