/**
 * TanStack Query hooks for automations.
 *
 * Queries name the tables they read, so `DbChanged` refreshes them after
 * any write, including the `automations` pipeline stage's: a run shows its
 * steps as they finish. Mutations also invalidate the automation queries,
 * so a change shows at once.
 */
import { queryOptions, useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import type { Table } from '@/lib/ipc/generated/shared';

import { automationsApi } from './api';
import type { AutomationId, AutomationInput, MeetingId } from './types';

/** Automations and their runs. */
const AUTOMATION_TABLES = [
  'automations',
  'automation_runs',
  'automation_run_steps',
] as const satisfies readonly Table[];

/** Runs on meetings, which also read what conditions match on. */
const MATCH_TABLES = [
  ...AUTOMATION_TABLES,
  'meetings',
  'meeting_tags',
  'meeting_folders',
  'meeting_attendees',
] as const satisfies readonly Table[];

/** Query keys; every key starts with `automationKeys.all`. */
export const automationKeys = {
  all: ['automations'] as const,
  catalog: () => [...automationKeys.all, 'catalog'] as const,
  list: () => [...automationKeys.all, 'list'] as const,
  meeting: (meetingId: MeetingId) => [...automationKeys.all, 'meeting', meetingId] as const,
  matching: (id: AutomationId) => [...automationKeys.all, 'matching', id] as const,
};

/** Every automation. Also usable from route loaders. */
export const automationListQuery = () =>
  queryOptions({
    queryKey: automationKeys.list(),
    queryFn: automationsApi.list,
    meta: { tables: AUTOMATION_TABLES },
  });

/** Every automation, oldest first. */
export function useAutomations() {
  return useQuery(automationListQuery());
}

/** The step picker (fixed for the app's lifetime). */
export function useStepCatalog() {
  return useQuery({
    queryKey: automationKeys.catalog(),
    queryFn: automationsApi.catalog,
    staleTime: Infinity,
  });
}

/** A meeting's ⋯ → Automations menu. */
export function useMeetingAutomations(meetingId: MeetingId, enabled = true) {
  return useQuery({
    queryKey: automationKeys.meeting(meetingId),
    queryFn: () => automationsApi.forMeeting(meetingId),
    meta: { tables: MATCH_TABLES },
    enabled,
  });
}

/** The finished meetings an automation matches. */
export function useMatchingMeetings(id: AutomationId) {
  return useQuery({
    queryKey: automationKeys.matching(id),
    queryFn: () => automationsApi.matching(id),
    meta: { tables: MATCH_TABLES },
  });
}

/** Refreshes every automation query once `work` settles. */
function useAutomationMutation<Args, Result>(work: (args: Args) => Promise<Result>) {
  const client = useQueryClient();
  return useMutation({
    mutationFn: work,
    onSettled: () => client.invalidateQueries({ queryKey: automationKeys.all }),
  });
}

/** Creates (`id` null) or replaces an automation. */
export function useSaveAutomation() {
  return useAutomationMutation(
    ({ id, input }: { id: AutomationId | null; input: AutomationInput }) =>
      id === null ? automationsApi.create(input) : automationsApi.update(id, input),
  );
}

/** Deletes an automation. */
export function useDeleteAutomation() {
  return useAutomationMutation(automationsApi.delete);
}

/** Duplicates an automation. */
export function useDuplicateAutomation() {
  return useAutomationMutation(automationsApi.duplicate);
}

/** Runs an automation on picked meetings. */
export function useRunAutomation() {
  return useAutomationMutation(
    ({ id, meetingIds }: { id: AutomationId; meetingIds: MeetingId[] }) =>
      automationsApi.run(id, meetingIds),
  );
}

/** Runs an automation on the newest finished meeting. */
export function useTestAutomation() {
  return useAutomationMutation(automationsApi.testLatest);
}
