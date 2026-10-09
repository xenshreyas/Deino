/**
 * TanStack Query reads for folders. Each names the tables it reads, so
 * `DbChanged` refreshes it after any write, including the `auto_folder`
 * pipeline stage's. Mutations are in `mutations.ts`.
 */
import { queryOptions, useQuery } from '@tanstack/react-query';

import type { Table } from '@/lib/ipc/generated/shared';

import { foldersApi } from './api';
import type { FolderId, MeetingId } from './types';

/** What the tree and the meeting lists read. */
const FILING_TABLES = [
  'folders',
  'meeting_folders',
  'meetings',
] as const satisfies readonly Table[];
/** What one folder reads. */
const FOLDER_TABLES = ['folders'] as const satisfies readonly Table[];
/** What a folder's rules read (series titles come from meetings, names from tags). */
const RULE_TABLES = [
  'folder_rules',
  'folders',
  'meetings',
  'tags',
] as const satisfies readonly Table[];

/** Query keys; every key starts with `folderKeys.all`. */
export const folderKeys = {
  all: ['folders'] as const,
  tree: () => [...folderKeys.all, 'tree'] as const,
  detail: (id: FolderId) => [...folderKeys.all, 'detail', id] as const,
  meetings: (id: FolderId) => [...folderKeys.all, 'meetings', id] as const,
  unfiled: () => [...folderKeys.all, 'unfiled'] as const,
  of: (meetingId: MeetingId) => [...folderKeys.all, 'of', meetingId] as const,
  rules: (id: FolderId) => [...folderKeys.all, 'rules', id] as const,
};

/** The folder tree in display order. Also usable from route loaders. */
export const folderTreeQuery = () =>
  queryOptions({
    queryKey: folderKeys.tree(),
    queryFn: foldersApi.tree,
    meta: { tables: FILING_TABLES },
  });

/** One folder. Also usable from route loaders. */
export const folderQuery = (id: FolderId) =>
  queryOptions({
    queryKey: folderKeys.detail(id),
    queryFn: () => foldersApi.get(id),
    meta: { tables: FOLDER_TABLES },
  });

/** The folder tree in display order, with counts. */
export function useFolderTree() {
  return useQuery(folderTreeQuery());
}

/** One folder. */
export function useFolder(id: FolderId) {
  return useQuery(folderQuery(id));
}

/** The meetings in a folder, newest first. */
export function useFolderMeetings(id: FolderId) {
  return useQuery({
    queryKey: folderKeys.meetings(id),
    queryFn: () => foldersApi.meetings(id),
    meta: { tables: FILING_TABLES },
  });
}

/** The meetings in no folder, newest first. */
export function useUnfiledMeetings() {
  return useQuery({
    queryKey: folderKeys.unfiled(),
    queryFn: foldersApi.unfiled,
    meta: { tables: FILING_TABLES },
  });
}

/** The folders a meeting sits in, by name. */
export function useMeetingFolders(meetingId: MeetingId) {
  return useQuery({
    queryKey: folderKeys.of(meetingId),
    queryFn: () => foldersApi.foldersOf(meetingId),
    meta: { tables: FILING_TABLES },
  });
}

/** A folder's auto-add rules, oldest first. */
export function useFolderRules(id: FolderId) {
  return useQuery({
    queryKey: folderKeys.rules(id),
    queryFn: () => foldersApi.rules(id),
    meta: { tables: RULE_TABLES },
  });
}
