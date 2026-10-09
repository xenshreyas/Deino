/**
 * TanStack Query mutations for folders. Each refetches the folder queries
 * when it succeeds, so the sidebar tree and open pages update at once
 * (`DbChanged` follows and reconciles).
 */
import { useMutation, useQueryClient } from '@tanstack/react-query';

import { foldersApi } from './api';
import { folderKeys } from './queries';
import type { FolderId, FolderRule, FolderRuleId, MeetingId, NewFolder } from './types';

/** Refetches every folder query. */
function useInvalidateFolders() {
  const client = useQueryClient();
  return () => client.invalidateQueries({ queryKey: folderKeys.all });
}

/** Creates a folder. */
export function useCreateFolder() {
  const onSuccess = useInvalidateFolders();
  return useMutation({ mutationFn: (folder: NewFolder) => foldersApi.create(folder), onSuccess });
}

/** Renames a folder. */
export function useRenameFolder() {
  const onSuccess = useInvalidateFolders();
  return useMutation({
    mutationFn: ({ id, name }: { id: FolderId; name: string }) => foldersApi.rename(id, name),
    onSuccess,
  });
}

/** Sets or clears a folder's icon. */
export function useSetFolderIcon() {
  const onSuccess = useInvalidateFolders();
  return useMutation({
    mutationFn: ({ id, icon }: { id: FolderId; icon: string | null }) =>
      foldersApi.setIcon(id, icon),
    onSuccess,
  });
}

/** Sets what a folder is for. */
export function useSetFolderDescription() {
  const onSuccess = useInvalidateFolders();
  return useMutation({
    mutationFn: ({ id, description }: { id: FolderId; description: string }) =>
      foldersApi.setDescription(id, description),
    onSuccess,
  });
}

/** Input of {@link useMoveFolder}: `parentId` `null` is the top level. */
export interface MoveFolderInput {
  id: FolderId;
  parentId: FolderId | null;
}

/** Moves a folder, with everything inside it. */
export function useMoveFolder() {
  const onSuccess = useInvalidateFolders();
  return useMutation({
    mutationFn: ({ id, parentId }: MoveFolderInput) => foldersApi.moveTo(id, parentId),
    onSuccess,
  });
}

/** Deletes a folder and its subfolders; meetings stay. */
export function useDeleteFolder() {
  const onSuccess = useInvalidateFolders();
  return useMutation({ mutationFn: (id: FolderId) => foldersApi.delete(id), onSuccess });
}

/** Input of the filing mutations. */
export interface FilingInput {
  folderId: FolderId;
  meetingId: MeetingId;
}

/** Files a meeting into a folder; resolves to the series offer, if any. */
export function useAddMeetingToFolder() {
  const onSuccess = useInvalidateFolders();
  return useMutation({
    mutationFn: ({ folderId, meetingId }: FilingInput) =>
      foldersApi.addMeeting(folderId, meetingId),
    onSuccess,
  });
}

/** Takes a meeting out of a folder. */
export function useRemoveMeetingFromFolder() {
  const onSuccess = useInvalidateFolders();
  return useMutation({
    mutationFn: ({ folderId, meetingId }: FilingInput) =>
      foldersApi.removeMeeting(folderId, meetingId),
    onSuccess,
  });
}

/** Adds an auto-add rule to a folder. */
export function useAddFolderRule() {
  const onSuccess = useInvalidateFolders();
  return useMutation({
    mutationFn: ({ folderId, rule }: { folderId: FolderId; rule: FolderRule }) =>
      foldersApi.addRule(folderId, rule),
    onSuccess,
  });
}

/** Deletes an auto-add rule. */
export function useRemoveFolderRule() {
  const onSuccess = useInvalidateFolders();
  return useMutation({ mutationFn: (id: FolderRuleId) => foldersApi.removeRule(id), onSuccess });
}
