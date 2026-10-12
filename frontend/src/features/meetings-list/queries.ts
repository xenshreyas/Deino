/**
 * TanStack Query hooks for the Meetings page. Every query names the tables
 * it reads in `meta.tables`, so `DbChanged` refreshes it after any write;
 * mutations need no cache surgery of their own.
 */
import {
  infiniteQueryOptions,
  queryOptions,
  useInfiniteQuery,
  useMutation,
  useQuery,
} from '@tanstack/react-query';

import type { Table } from '@/lib/ipc/generated/shared';

import { meetingsListApi } from './api';
import { filterKey } from './filters';
import type { MeetingFilter, MeetingViewId, MeetingViewInput } from './types';

/** Rows per page: a screen and a half of 40px rows. */
export const PAGE_SIZE = 50;

/** Everything a list row reads. */
const LIST_TABLES = [
  'meetings',
  'meeting_attendees',
  'people',
  'meeting_tags',
  'tags',
  'meeting_folders',
  'folders',
  'action_items',
  'speakers',
] as const satisfies readonly Table[];

/** Query keys; every key starts with `meetingsListKeys.all`. */
export const meetingsListKeys = {
  all: ['meetings-list'] as const,
  pages: (filter: MeetingFilter) => [...meetingsListKeys.all, 'pages', filterKey(filter)] as const,
  views: () => [...meetingsListKeys.all, 'views'] as const,
  trash: () => [...meetingsListKeys.all, 'trash'] as const,
};

/** The list's pages for `filter`, loaded one cursor at a time. */
export const meetingPagesQuery = (filter: MeetingFilter) =>
  infiniteQueryOptions({
    queryKey: meetingsListKeys.pages(filter),
    queryFn: ({ pageParam }) => meetingsListApi.page(filter, pageParam, PAGE_SIZE),
    initialPageParam: null as Parameters<typeof meetingsListApi.page>[1],
    getNextPageParam: (page) => page.next,
    meta: { tables: LIST_TABLES },
  });

/** The saved views, in tab order. */
export const meetingViewsQuery = () =>
  queryOptions({
    queryKey: meetingsListKeys.views(),
    queryFn: meetingsListApi.views,
    meta: { tables: ['meeting_views'] satisfies Table[] },
  });

/** The trash. */
export const trashQuery = () =>
  queryOptions({
    queryKey: meetingsListKeys.trash(),
    queryFn: meetingsListApi.trashed,
    meta: { tables: ['meetings'] satisfies Table[] },
  });

/** The list's pages for `filter`. */
export function useMeetingPages(filter: MeetingFilter) {
  return useInfiniteQuery(meetingPagesQuery(filter));
}

/** The saved views. */
export function useMeetingViews() {
  return useQuery(meetingViewsQuery());
}

/** The trash. */
export function useTrash() {
  return useQuery(trashQuery());
}

/** Saves the view on screen as a new view (`id` null) or over `id`. */
export function useSaveView() {
  return useMutation({
    mutationFn: ({ id, view }: { id: MeetingViewId | null; view: MeetingViewInput }) =>
      id === null ? meetingsListApi.createView(view) : meetingsListApi.updateView(id, view),
  });
}

/** Deletes a saved view. */
export function useDeleteView() {
  return useMutation({ mutationFn: (id: MeetingViewId) => meetingsListApi.deleteView(id) });
}
