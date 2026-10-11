/**
 * TanStack Query hooks for speakers and voice profiles.
 *
 * Queries name the tables they read, so `DbChanged` refreshes them after any
 * write, including the `diarize` and `identify_speakers` stages'. Mutations
 * also invalidate the speaker queries themselves, so a rename shows at once.
 */
import {
  keepPreviousData,
  queryOptions,
  useMutation,
  useQuery,
  useQueryClient,
} from '@tanstack/react-query';

import type { Table } from '@/lib/ipc/generated/shared';

import { speakersApi } from './api';
import type { MeetingId, PersonId, Rename, SpeakerId, VoiceprintId } from './types';

/** The tables the participants panel reads. */
const PANEL_TABLES = [
  'speakers',
  'transcript_segments',
  'people',
  'speaker_suggestions',
] as const satisfies readonly Table[];
/** The tables the name picker reads. */
const CANDIDATE_TABLES = ['people', 'meeting_attendees'] as const satisfies readonly Table[];
/** The tables the voice profiles read. */
const PROFILE_TABLES = ['voiceprints', 'people', 'meetings'] as const satisfies readonly Table[];

/** Query keys; every key starts with `speakerKeys.all`. */
export const speakerKeys = {
  all: ['speakers'] as const,
  participants: (meetingId: MeetingId) => [...speakerKeys.all, 'participants', meetingId] as const,
  candidates: (meetingId: MeetingId, query: string) =>
    [...speakerKeys.all, 'candidates', meetingId, query] as const,
  profiles: () => [...speakerKeys.all, 'profiles'] as const,
};

/** A meeting's participants, most speech first. */
export const participantsQuery = (meetingId: MeetingId) =>
  queryOptions({
    queryKey: speakerKeys.participants(meetingId),
    queryFn: () => speakersApi.participants(meetingId),
    meta: { tables: PANEL_TABLES },
  });

/** A meeting's participants, most speech first. */
export function useParticipants(meetingId: MeetingId) {
  return useQuery(participantsQuery(meetingId));
}

/** People to name a speaker after; the previous list stays while typing. */
export function useNameCandidates(meetingId: MeetingId, query: string) {
  return useQuery({
    queryKey: speakerKeys.candidates(meetingId, query.trim()),
    queryFn: () => speakersApi.candidates(meetingId, query.trim()),
    meta: { tables: CANDIDATE_TABLES },
    placeholderData: keepPreviousData,
  });
}

/** Everyone with a voice profile. */
export function useVoiceProfiles() {
  return useQuery({
    queryKey: speakerKeys.profiles(),
    queryFn: speakersApi.voiceProfiles,
    meta: { tables: PROFILE_TABLES },
  });
}

/** Refetches every speaker query once a mutation settles. */
function useInvalidateSpeakers() {
  const client = useQueryClient();
  return () => client.invalidateQueries({ queryKey: speakerKeys.all });
}

/** Names a speaker for one segment or all of them. */
export function useRenameSpeaker() {
  const invalidate = useInvalidateSpeakers();
  return useMutation({
    mutationFn: (rename: Rename) => speakersApi.rename(rename),
    onSuccess: invalidate,
  });
}

/** Input of {@link useMergeSpeakers}. */
export interface MergeInput {
  from: SpeakerId;
  into: SpeakerId;
}

/** Folds one speaker into another. */
export function useMergeSpeakers(meetingId: MeetingId) {
  const invalidate = useInvalidateSpeakers();
  return useMutation({
    mutationFn: ({ from, into }: MergeInput) => speakersApi.merge(meetingId, from, into),
    onSuccess: invalidate,
  });
}

/** "Yes": names the speaker after its suggestion. */
export function useConfirmSuggestion(meetingId: MeetingId) {
  const invalidate = useInvalidateSpeakers();
  return useMutation({
    mutationFn: (speakerId: SpeakerId) => speakersApi.confirmSuggestion(meetingId, speakerId),
    onSuccess: invalidate,
  });
}

/** "No": forgets the speaker's suggestion. */
export function useDismissSuggestion() {
  const invalidate = useInvalidateSpeakers();
  return useMutation({
    mutationFn: (speakerId: SpeakerId) => speakersApi.dismissSuggestion(speakerId),
    onSuccess: invalidate,
  });
}

/** Deletes one voice sample. */
export function useDeleteVoiceprint() {
  const invalidate = useInvalidateSpeakers();
  return useMutation({
    mutationFn: (id: VoiceprintId) => speakersApi.deleteVoiceprint(id),
    onSuccess: invalidate,
  });
}

/** Deletes a person's whole voice profile. */
export function useForgetVoice() {
  const invalidate = useInvalidateSpeakers();
  return useMutation({
    mutationFn: (personId: PersonId) => speakersApi.forgetVoice(personId),
    onSuccess: invalidate,
  });
}
