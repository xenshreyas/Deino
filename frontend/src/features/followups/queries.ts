/**
 * TanStack Query hooks for suggested follow-ups. Queries name the
 * `follow_up_drafts` table (and `meetings`, for titles and the trash), so
 * `DbChanged` refreshes them when the pipeline writes a draft.
 */
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import type { MeetingId } from '@/lib/ipc/generated/meetings';
import type { Table } from '@/lib/ipc/generated/shared';

import { followUpsApi } from './api';
import type { FollowUpEdit, FollowUpSuggestion } from './types';

/** The tables a follow-up reads. */
const FOLLOW_UP_TABLES = ['follow_up_drafts', 'meetings'] as const satisfies readonly Table[];

/** Query keys; every key starts with `followUpKeys.all`. */
export const followUpKeys = {
  all: ['followups'] as const,
  suggested: () => [...followUpKeys.all, 'suggested'] as const,
  meeting: (meeting: MeetingId) => [...followUpKeys.all, 'meeting', meeting] as const,
};

/** Home's "Suggested follow-ups". */
export function useSuggestedFollowUps() {
  return useQuery({
    queryKey: followUpKeys.suggested(),
    queryFn: followUpsApi.suggested,
    meta: { tables: FOLLOW_UP_TABLES },
  });
}

/** ORG-02's Home slot: one entry per ready draft, titled by its meeting. */
export function useFollowUpSuggestions(): readonly FollowUpSuggestion[] {
  const suggested = useSuggestedFollowUps();
  return (suggested.data ?? []).map((summary) => ({
    id: summary.meetingId,
    title: summary.meetingTitle,
    meetingId: summary.meetingId,
  }));
}

/** The meeting's ready draft, or `null`. */
export function useMeetingFollowUp(meeting: MeetingId) {
  return useQuery({
    queryKey: followUpKeys.meeting(meeting),
    queryFn: () => followUpsApi.forMeeting(meeting),
    meta: { tables: FOLLOW_UP_TABLES },
  });
}

/** Refetches every follow-up query once a mutation settles. */
function useInvalidateFollowUps() {
  const client = useQueryClient();
  return () => client.invalidateQueries({ queryKey: followUpKeys.all });
}

/** What the editor's actions send: the meeting and its fields. */
interface EditRequest {
  meeting: MeetingId;
  edit: FollowUpEdit;
}

/** Saves an edit (closing the editor with changes). */
export function useSaveFollowUp() {
  const invalidate = useInvalidateFollowUps();
  return useMutation({
    mutationFn: ({ meeting, edit }: EditRequest) => followUpsApi.save(meeting, edit),
    onSettled: invalidate,
  });
}

/** Saves, then opens the draft in Mail. */
export function useOpenInMail() {
  const invalidate = useInvalidateFollowUps();
  return useMutation({
    mutationFn: ({ meeting, edit }: EditRequest) => followUpsApi.openInMail(meeting, edit),
    onSettled: invalidate,
  });
}

/** Saves, then copies the body. */
export function useCopyFollowUp() {
  const invalidate = useInvalidateFollowUps();
  return useMutation({
    mutationFn: ({ meeting, edit }: EditRequest) => followUpsApi.copy(meeting, edit),
    onSettled: invalidate,
  });
}

/** Dismisses a follow-up for good. */
export function useDismissFollowUp() {
  const invalidate = useInvalidateFollowUps();
  return useMutation({ mutationFn: followUpsApi.dismiss, onSettled: invalidate });
}
