/**
 * TanStack Query hooks for the example feature.
 *
 * Queries name the tables they read (`meta.tables`), so Rust's `DbChanged`
 * events keep them fresh with no manual invalidation. A mutation writes its
 * result into the cache for an instant update; the event that follows reconciles.
 */
import { queryOptions, useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import type { Table } from '@/lib/ipc/generated/shared';

import { exampleApi } from './api';
import type { ExampleNote } from './types';

/** The tables the example's queries read. */
const EXAMPLE_TABLES = ['example_notes'] as const satisfies readonly Table[];

/** Query keys; every key starts with `exampleKeys.all`. */
export const exampleKeys = {
  all: ['example'] as const,
  notes: () => [...exampleKeys.all, 'notes'] as const,
};

/** Every note, newest first. */
export const exampleNotesQuery = () =>
  queryOptions({
    queryKey: exampleKeys.notes(),
    queryFn: exampleApi.list,
    meta: { tables: EXAMPLE_TABLES },
  });

/** Every note, newest first. */
export function useExampleNotes() {
  return useQuery(exampleNotesQuery());
}

/** Saves a note and puts it at the top of the cached list. */
export function useAddExampleNote() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: exampleApi.add,
    onSuccess: (note) => {
      client.setQueryData(exampleKeys.notes(), (notes: ExampleNote[] | undefined) => [
        note,
        ...(notes ?? []),
      ]);
    },
  });
}
