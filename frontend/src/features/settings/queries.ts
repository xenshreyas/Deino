/**
 * TanStack Query hooks for settings and Keychain key status.
 *
 * Settings live in a file, not a table, so `DbChanged` never covers them.
 * Instead every saved change arrives as `SettingsChanged`, which
 * `useSettingsChanges` writes straight into the cache, and every mutation
 * writes its result too: the cached settings are always the saved ones.
 */
import {
  queryOptions,
  type QueryClient,
  skipToken,
  useMutation,
  useQuery,
  useQueryClient,
} from '@tanstack/react-query';

import { events } from '@/lib/ipc/generated/settings';
import { useIpcEvent } from '@/lib/ipc/events';

import { secretsApi, settingsApi } from './api';
import type { SecretName, SecretStatus, Settings } from './types';

/** Query keys; every key starts with `settingsKeys.all`. */
export const settingsKeys = {
  all: ['settings'] as const,
  document: () => [...settingsKeys.all, 'document'] as const,
  secret: (name: SecretName | null) => [...settingsKeys.all, 'secret', name] as const,
};

/** Every setting, kept fresh by `SettingsChanged`. */
export const settingsQuery = () =>
  queryOptions({ queryKey: settingsKeys.document(), queryFn: settingsApi.get });

/**
 * Key status is re-read on every mount: the Keychain can lose access behind
 * the app's back (an ad-hoc build's new signature).
 */
const SECRET_STATUS_STALE_TIME = 0;

/** Whether a key is saved. */
export const secretStatusQuery = (name: SecretName) =>
  queryOptions({
    queryKey: settingsKeys.secret(name),
    queryFn: () => secretsApi.status(name),
    staleTime: SECRET_STATUS_STALE_TIME,
  });

/** Every setting. */
export function useSettings() {
  return useQuery(settingsQuery());
}

/** Applies a merge patch and caches the saved settings it returns. */
export function usePatchSettings() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: settingsApi.patch,
    onSuccess: (settings) => {
      cacheSettings(client, settings);
    },
  });
}

/**
 * Whether the key `name` is `missing`, `stored` or `locked`; idle when
 * `name` is `null` (a provider without a key).
 */
export function useSecretStatus(name: SecretName | null) {
  return useQuery({
    queryKey: settingsKeys.secret(name),
    queryFn: name === null ? skipToken : () => secretsApi.status(name),
    staleTime: SECRET_STATUS_STALE_TIME,
  });
}

/** Input of {@link useSaveSecret}. */
export interface SaveSecretInput {
  name: SecretName;
  value: string;
}

/** Saves a key in the Keychain. */
export function useSaveSecret() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: ({ name, value }: SaveSecretInput) => secretsApi.set(name, value),
    onSuccess: (_saved, { name }) => {
      cacheStatus(client, name, 'stored');
    },
  });
}

/** Forgets a key. */
export function useDeleteSecret() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: secretsApi.delete,
    onSuccess: (_deleted, name) => {
      cacheStatus(client, name, 'missing');
    },
  });
}

/** Writes every `SettingsChanged` into the cache while mounted. Mount it once. */
export function useSettingsChanges(): void {
  const client = useQueryClient();
  useIpcEvent(events.settingsChanged, ({ settings }) => {
    cacheSettings(client, settings);
  });
}

function cacheSettings(client: QueryClient, settings: Settings): void {
  client.setQueryData(settingsKeys.document(), settings);
}

function cacheStatus(client: QueryClient, name: SecretName, status: SecretStatus): void {
  client.setQueryData(settingsKeys.secret(name), status);
}
