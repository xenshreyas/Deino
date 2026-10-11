/**
 * TanStack Query hooks for the vocabulary.
 *
 * Queries name the tables they read, so `DbChanged` refreshes them after any
 * write, the `vocab_fixups` stage's included. Mutations also invalidate the
 * vocabulary queries themselves, so an edit shows at once.
 */
import { queryOptions, useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import type { Table } from '@/lib/ipc/generated/shared';

import { vocabularyApi } from './api';
import type { MeetingId, SegmentId, TermInput, VocabTermId } from './types';

/** The tables the entries are read from. */
const TERM_TABLES = ['vocab_terms', 'vocab_variants'] as const satisfies readonly Table[];
/** The tables a meeting's corrections are read from. */
const CORRECTION_TABLES = [
  'vocab_corrections',
  'transcript_segments',
] as const satisfies readonly Table[];

/** Query keys; every key starts with `vocabularyKeys.all`. */
export const vocabularyKeys = {
  all: ['vocabulary'] as const,
  terms: () => [...vocabularyKeys.all, 'terms'] as const,
  corrections: (meetingId: MeetingId) => [...vocabularyKeys.all, 'corrections', meetingId] as const,
};

/** Every entry. */
export const vocabularyQuery = () =>
  queryOptions({
    queryKey: vocabularyKeys.terms(),
    queryFn: vocabularyApi.list,
    meta: { tables: TERM_TABLES },
  });

/** Every entry. */
export function useVocabulary() {
  return useQuery(vocabularyQuery());
}

/** A meeting's corrections that are not undone. */
export function useCorrections(meetingId: MeetingId) {
  return useQuery({
    queryKey: vocabularyKeys.corrections(meetingId),
    queryFn: () => vocabularyApi.corrections(meetingId),
    meta: { tables: CORRECTION_TABLES },
  });
}

/** Refetches every vocabulary query once a mutation settles. */
function useInvalidate() {
  const client = useQueryClient();
  return () => client.invalidateQueries({ queryKey: vocabularyKeys.all });
}

/** Adds an entry. */
export function useCreateTerm() {
  const onSettled = useInvalidate();
  return useMutation({ mutationFn: (input: TermInput) => vocabularyApi.create(input), onSettled });
}

/** Replaces an entry. */
export function useUpdateTerm() {
  const onSettled = useInvalidate();
  return useMutation({
    mutationFn: ({ id, input }: { id: VocabTermId; input: TermInput }) =>
      vocabularyApi.update(id, input),
    onSettled,
  });
}

/** Deletes an entry. */
export function useDeleteTerm() {
  const onSettled = useInvalidate();
  return useMutation({ mutationFn: (id: VocabTermId) => vocabularyApi.delete(id), onSettled });
}

/** Undoes one segment's correction (`segmentId`), or all of the meeting's (`null`). */
export function useUndoCorrections(meetingId: MeetingId) {
  const onSettled = useInvalidate();
  return useMutation({
    mutationFn: (segmentId: SegmentId | null) => vocabularyApi.undo(meetingId, segmentId),
    onSettled,
  });
}
