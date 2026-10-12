/**
 * TanStack Query over the hotkeys. They live in the settings file, not the
 * database, so the list is refreshed from each change's answer.
 */
import { queryOptions, useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import { hotkeysApi } from './api';
import type { Hotkey, HotkeyAction } from './types';

/** Query keys; every key starts with `hotkeyKeys.all`. */
export const hotkeyKeys = {
  all: ['hotkeys'] as const,
};

/** Every hotkey. */
export const hotkeysQuery = () =>
  queryOptions({ queryKey: hotkeyKeys.all, queryFn: hotkeysApi.list });

/** The hotkeys and their states. */
export function useHotkeys() {
  return useQuery(hotkeysQuery());
}

/** One change: `accelerator` is `null` to turn the hotkey off. */
export interface HotkeyChange {
  action: HotkeyAction;
  accelerator: string | null;
}

/** Changes one hotkey; the answer replaces the list. */
export function useSetHotkey() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ action, accelerator }: HotkeyChange) => hotkeysApi.set(action, accelerator),
    onSuccess: (hotkeys: Hotkey[]) => {
      queryClient.setQueryData(hotkeyKeys.all, hotkeys);
    },
  });
}
