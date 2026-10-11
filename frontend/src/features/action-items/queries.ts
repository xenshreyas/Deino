/**
 * TanStack Query hooks for action items.
 *
 * Queries name the tables they read in `meta.tables`, so `DbChanged`
 * refreshes them after any write, from this page, the Notes tab, the
 * pipeline or another window. Edits are applied to the cached lists at once
 * (a ticked box flips before the round trip) and reconciled by the refresh.
 */
import {
  type QueryClient,
  queryOptions,
  useMutation,
  useQuery,
  useQueryClient,
} from '@tanstack/react-query';

import type { Table } from '@/lib/ipc/generated/shared';

import { actionItemsApi } from './api';
import { filterKey } from './filters';
import type {
  ActionItemEdit,
  ActionItemFilter,
  ActionItemId,
  ActionItemView,
  ActionItemViewId,
  SavedViewInput,
} from './types';

/** Everything a list row reads: the item, its meeting, assignee and evidence. */
const LIST_TABLES = [
  'action_items',
  'meetings',
  'people',
  'transcript_segments',
  'meeting_tags',
  'meeting_folders',
  'meeting_attendees',
  'folders',
] as const satisfies readonly Table[];
/** What the filter bar offers. */
const OPTION_TABLES = [
  'people',
  'companies',
  'tags',
  'folders',
] as const satisfies readonly Table[];
/** The saved views. */
const VIEW_TABLES = ['action_item_views'] as const satisfies readonly Table[];

/** Query keys; every key starts with `actionItemKeys.all`. */
export const actionItemKeys = {
  all: ['action-items'] as const,
  lists: () => [...actionItemKeys.all, 'list'] as const,
  list: (filter: ActionItemFilter) => [...actionItemKeys.lists(), filterKey(filter)] as const,
  today: () => [...actionItemKeys.all, 'today'] as const,
  options: () => [...actionItemKeys.all, 'options'] as const,
  views: () => [...actionItemKeys.all, 'views'] as const,
};

/** The items `filter` matches. */
export const actionItemListQuery = (filter: ActionItemFilter) =>
  queryOptions({
    queryKey: actionItemKeys.list(filter),
    queryFn: () => actionItemsApi.list(filter),
    meta: { tables: LIST_TABLES },
  });

/** The "Today" digest. */
export const todayQuery = () =>
  queryOptions({
    queryKey: actionItemKeys.today(),
    queryFn: actionItemsApi.today,
    meta: { tables: LIST_TABLES },
  });

/** The filter bar's and pickers' options. */
export const filterOptionsQuery = () =>
  queryOptions({
    queryKey: actionItemKeys.options(),
    queryFn: actionItemsApi.filterOptions,
    meta: { tables: OPTION_TABLES },
  });

/** The saved views. */
export const savedViewsQuery = () =>
  queryOptions({
    queryKey: actionItemKeys.views(),
    queryFn: actionItemsApi.views,
    meta: { tables: VIEW_TABLES },
  });

/** The items `filter` matches. */
export function useActionItems(filter: ActionItemFilter) {
  return useQuery(actionItemListQuery(filter));
}

/** The user's overdue and due-today items. */
export function useToday() {
  return useQuery(todayQuery());
}

/** People, companies, tags and folders. */
export function useFilterOptions() {
  return useQuery(filterOptionsQuery());
}

/** The saved views, in tab order. */
export function useSavedViews() {
  return useQuery(savedViewsQuery());
}

/** `item` with `edit` applied the way Rust applies it, for an instant update. */
export function applyEdit(item: ActionItemView, edit: ActionItemEdit, now: Date): ActionItemView {
  switch (edit.field) {
    case 'status':
      return {
        ...item,
        status: edit.status,
        completedAt: edit.status === 'done' ? (item.completedAt ?? now.toISOString()) : null,
      };
    case 'title':
      return { ...item, title: edit.title.trim() };
    case 'description':
      return { ...item, description: edit.description.trim() };
    case 'priority':
      return { ...item, priority: edit.priority };
    case 'assignee':
    case 'due':
      // Names and local noon come from Rust; the refresh fills them in.
      return item;
  }
}

/** Applies `edit` to the cached copies of `ids` in every list and the digest. */
function patchCaches(client: QueryClient, ids: readonly ActionItemId[], edit: ActionItemEdit) {
  const now = new Date();
  const patch = (item: ActionItemView) =>
    ids.includes(item.id) ? applyEdit(item, edit, now) : item;
  client.setQueriesData<ActionItemView[]>({ queryKey: actionItemKeys.lists() }, (items) =>
    items?.map(patch),
  );
}

/** Applies one edit to one item, at once in every cached list. */
export function useUpdateActionItem() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: ({ id, edit }: { id: ActionItemId; edit: ActionItemEdit }) =>
      actionItemsApi.update(id, edit),
    onMutate: async ({ id, edit }) => {
      await client.cancelQueries({ queryKey: actionItemKeys.lists() });
      patchCaches(client, [id], edit);
    },
    onError: () => client.invalidateQueries({ queryKey: actionItemKeys.all }),
  });
}

/** Applies a status, assignee, due or priority edit to many items. */
export function useBulkUpdate() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: ({ ids, edit }: { ids: ActionItemId[]; edit: ActionItemEdit }) =>
      actionItemsApi.bulkUpdate(ids, edit),
    onMutate: async ({ ids, edit }) => {
      await client.cancelQueries({ queryKey: actionItemKeys.lists() });
      patchCaches(client, ids, edit);
    },
    onError: () => client.invalidateQueries({ queryKey: actionItemKeys.all }),
  });
}

/** Adds an item the user wrote. */
export function useCreateActionItem() {
  return useMutation({ mutationFn: actionItemsApi.create });
}

/** Deletes items. */
export function useDeleteActionItems() {
  return useMutation({ mutationFn: actionItemsApi.delete });
}

/** Completes every open item of the user's. */
export function useMarkAllMineDone() {
  return useMutation({ mutationFn: actionItemsApi.markAllMineDone });
}

/** Saves a new view, or the name and filter of an existing one. */
export function useSaveView() {
  return useMutation({
    mutationFn: ({ id, view }: { id: ActionItemViewId | null; view: SavedViewInput }) =>
      id === null ? actionItemsApi.createView(view) : actionItemsApi.updateView(id, view),
  });
}

/** Deletes a saved view. */
export function useDeleteView() {
  return useMutation({ mutationFn: actionItemsApi.deleteView });
}
