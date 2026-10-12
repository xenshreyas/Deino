/**
 * TanStack Query hooks for onboarding: the saved progress (a slice of the
 * settings document) with its step machine, and the live permissions.
 */
import { queryOptions, useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useCallback } from 'react';

import { type Settings, settingsApi, settingsKeys, useSettings } from '@/features/settings';

import { onboardingApi } from './api';
import { type OnboardingAction, transition } from './machine';
import type { OnboardingSettings, PermissionStatus } from './types';

/** Query keys; every key starts with `onboardingKeys.all`. */
export const onboardingKeys = {
  all: ['onboarding'] as const,
  permissions: () => [...onboardingKeys.all, 'permissions'] as const,
};

/**
 * How often the permissions are re-read while they are on screen, so a
 * switch flipped in System Settings shows up without a click.
 */
const PERMISSIONS_POLL_MS = 1500;

/** Every permission, re-read while mounted and whenever the app regains focus. */
export const permissionsQuery = () =>
  queryOptions({
    queryKey: onboardingKeys.permissions(),
    queryFn: onboardingApi.permissions,
    refetchInterval: PERMISSIONS_POLL_MS,
    refetchOnWindowFocus: 'always',
  });

/** Every permission onboarding asks for. */
export function usePermissions() {
  return useQuery(permissionsQuery());
}

/** Writes one permission's new status into the cached list. */
function withStatus(list: PermissionStatus[] | undefined, status: PermissionStatus) {
  return list?.map((item) => (item.kind === status.kind ? status : item));
}

/** Asks for a permission; the answer replaces its cached status. */
export function useRequestPermission() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: onboardingApi.requestPermission,
    onSuccess: (status) => {
      client.setQueryData(onboardingKeys.permissions(), (list: PermissionStatus[] | undefined) =>
        withStatus(list, status),
      );
    },
  });
}

/** Opens a permission's System Settings page. */
export function useOpenPermissionSettings() {
  return useMutation({ mutationFn: onboardingApi.openPermissionSettings });
}

/** The saved onboarding progress; `undefined` until the settings load. */
export function useOnboardingProgress(): OnboardingSettings | undefined {
  return useSettings().data?.onboarding;
}

/** Saves a progress, showing it at once and rolling back if the save fails. */
function useSaveProgress() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: (onboarding: OnboardingSettings) => settingsApi.patch({ onboarding }),
    onMutate: (onboarding) => {
      const before = client.getQueryData<Settings>(settingsKeys.document());
      if (before) client.setQueryData(settingsKeys.document(), { ...before, onboarding });
      return before;
    },
    onError: (_error, _onboarding, before) => {
      if (before) client.setQueryData(settingsKeys.document(), before);
    },
    onSuccess: (settings) => {
      client.setQueryData(settingsKeys.document(), settings);
    },
  });
}

/**
 * Moves the flow: applies `action` to the cached progress with the step
 * machine and saves the result. Does nothing before the settings load.
 */
export function useOnboardingDispatch(): (action: OnboardingAction) => void {
  const client = useQueryClient();
  const { mutate } = useSaveProgress();
  return useCallback(
    (action: OnboardingAction) => {
      const settings = client.getQueryData<Settings>(settingsKeys.document());
      if (settings) mutate(transition(settings.onboarding, action));
    },
    [client, mutate],
  );
}
