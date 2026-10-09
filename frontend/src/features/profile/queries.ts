/**
 * TanStack Query hooks for the profile. Saving rewrites the settings file,
 * whose `SettingsChanged` event refreshes `useSettings()`; the AI context
 * preview is refetched after each save.
 */
import { queryOptions, useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import { profileApi } from './api';

/** Query keys; every key starts with `profileKeys.all`. */
export const profileKeys = {
  all: ['profile'] as const,
  context: () => [...profileKeys.all, 'context'] as const,
};

/** The context every AI prompt starts with. */
export const profileContextQuery = () =>
  queryOptions({ queryKey: profileKeys.context(), queryFn: profileApi.context });

/** The context every AI prompt starts with. */
export function useProfileContext() {
  return useQuery(profileContextQuery());
}

/** Saves the profile and refreshes the context preview. */
export function useSaveProfile() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: profileApi.update,
    onSuccess: () => client.invalidateQueries({ queryKey: profileKeys.context() }),
  });
}
