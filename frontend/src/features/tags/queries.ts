/**
 * TanStack Query hooks for tags.
 *
 * Queries name the tables they read, so `DbChanged` refreshes them after any
 * write, including the `auto_tag` pipeline stage's. Mutations also
 * invalidate the tag queries themselves, so the change shows at once.
 */
import { queryOptions, useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import type { MeetingId } from '@/lib/ipc/generated/meetings';
import type { Table } from '@/lib/ipc/generated/shared';

import { tagsApi } from './api';
import type { NewTag, Tag, TagId, TagUpdate } from './types';

/** The tables the tag list reads. */
const TAG_TABLES = ['tags'] as const satisfies readonly Table[];
/** The tables a meeting's tags and the tag links read. */
const APPLIED_TABLES = ['tags', 'meeting_tags', 'meetings'] as const satisfies readonly Table[];

/** Query keys; every key starts with `tagKeys.all`. */
export const tagKeys = {
  all: ['tags'] as const,
  list: () => [...tagKeys.all, 'list'] as const,
  of: (meetingId: MeetingId) => [...tagKeys.all, 'of', meetingId] as const,
  links: () => [...tagKeys.all, 'links'] as const,
};

/** Every tag, by name. Also usable from route loaders. */
export const tagListQuery = () =>
  queryOptions({ queryKey: tagKeys.list(), queryFn: tagsApi.list, meta: { tables: TAG_TABLES } });

/** Every tag, by name. */
export function useTags() {
  return useQuery(tagListQuery());
}

/** The tags on one meeting, with who applied them. */
export function useMeetingTags(meetingId: MeetingId) {
  return useQuery({
    queryKey: tagKeys.of(meetingId),
    queryFn: () => tagsApi.tagsOf(meetingId),
    meta: { tables: APPLIED_TABLES },
  });
}

/** Every tag on every meeting. */
export function useTagLinks() {
  return useQuery({
    queryKey: tagKeys.links(),
    queryFn: tagsApi.links,
    meta: { tables: APPLIED_TABLES },
  });
}

/** Refetches every tag query once a mutation settles. */
function useInvalidateTags() {
  const client = useQueryClient();
  return () => client.invalidateQueries({ queryKey: tagKeys.all });
}

/** Creates a tag. */
export function useCreateTag() {
  const invalidate = useInvalidateTags();
  return useMutation({ mutationFn: (tag: NewTag) => tagsApi.create(tag), onSuccess: invalidate });
}

/** Input of {@link useUpdateTag}. */
export interface UpdateTagInput {
  id: TagId;
  update: TagUpdate;
}

/** Replaces a tag's editable fields. */
export function useUpdateTag() {
  const invalidate = useInvalidateTags();
  return useMutation({
    mutationFn: ({ id, update }: UpdateTagInput) => tagsApi.update(id, update),
    onSuccess: invalidate,
  });
}

/** Deletes a tag. */
export function useDeleteTag() {
  const invalidate = useInvalidateTags();
  return useMutation({ mutationFn: (id: TagId) => tagsApi.delete(id), onSuccess: invalidate });
}

/** Input of the apply and remove mutations. */
export interface MeetingTagInput {
  meetingId: MeetingId;
  tagId: TagId;
}

/** Puts a tag on a meeting by hand. */
export function useApplyTag() {
  const invalidate = useInvalidateTags();
  return useMutation({
    mutationFn: ({ meetingId, tagId }: MeetingTagInput) => tagsApi.apply(meetingId, tagId),
    onSuccess: invalidate,
  });
}

/** Takes a tag off a meeting. */
export function useRemoveTag() {
  const invalidate = useInvalidateTags();
  return useMutation({
    mutationFn: ({ meetingId, tagId }: MeetingTagInput) => tagsApi.remove(meetingId, tagId),
    onSuccess: invalidate,
  });
}

/** Creates a tag from the combobox's search and puts it on the meeting. */
export function useCreateAndApplyTag() {
  const invalidate = useInvalidateTags();
  return useMutation({
    mutationFn: async ({ meetingId, name }: { meetingId: MeetingId; name: string }) => {
      const tag: Tag = await tagsApi.create({ name, color: null, description: '' });
      await tagsApi.apply(meetingId, tag.id);
      return tag;
    },
    onSuccess: invalidate,
  });
}
