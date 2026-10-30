/**
 * Notes being written, as they are written (streamed notes): the latest
 * draft of the meeting's run, from Rust's `NotesRun` event, with the
 * `draft` command for a page opened mid-run.
 *
 * The cache entry outlives the run briefly: once the notes are saved it
 * keeps the last draft until the new version reaches the page (so the old
 * notes never flash back), and a stopped run leaves a note saying so.
 */
import {
  type QueryClient,
  queryOptions,
  useMutation,
  useQuery,
  useQueryClient,
} from '@tanstack/react-query';

import type { MeetingId } from '@/features/meetings';
import { events } from '@/lib/ipc/generated/enhance';
import { useIpcEvent } from '@/lib/ipc/events';

import { enhanceApi } from '../api';
import { enhanceKeys } from '../queries';
import type { EnhancedNotesView, NotesDraft, NotesRunUpdate } from '../types';

/** What the Enhanced view knows of the meeting's latest run. */
export type LiveNotes =
  | { kind: 'writing'; draft: NotesDraft }
  | { kind: 'written'; draft: NotesDraft; version: number; kept: number }
  | { kind: 'stopped' };

/**
 * The run in progress, read on mount; events keep it current after. An
 * event that lands while the read is in flight wins over the read.
 */
export const liveNotesQuery = (client: QueryClient, id: MeetingId) =>
  queryOptions({
    queryKey: enhanceKeys.live(id),
    queryFn: async (): Promise<LiveNotes | null> => {
      const asked = Date.now();
      const draft = await enhanceApi.draft(id);
      const state = client.getQueryState<LiveNotes | null>(enhanceKeys.live(id));
      if (state !== undefined && state.dataUpdatedAt >= asked) return state.data ?? null;
      return draft === null ? null : { kind: 'writing', draft };
    },
    staleTime: Infinity,
    refetchOnMount: 'always',
    refetchOnWindowFocus: false,
  });

/** The live state after `update`. */
export function nextLive(
  previous: LiveNotes | null | undefined,
  update: NotesRunUpdate,
): LiveNotes | null {
  if (update.kind === 'draft') return { kind: 'writing', draft: update.draft };
  const { end } = update;
  if (end.kind === 'stopped') return { kind: 'stopped' };
  if (end.kind === 'failed' || previous === null || previous === undefined) return null;
  if (previous.kind === 'stopped') return null;
  return { kind: 'written', draft: previous.draft, version: end.version, kept: end.kept };
}

/** The draft to show instead of the notes, or `null` when the notes are current. */
export function shownDraft(
  live: LiveNotes | null | undefined,
  view: EnhancedNotesView | null,
): NotesDraft | null {
  if (live === null || live === undefined || live.kind === 'stopped') return null;
  if (live.kind === 'writing') return live.draft;
  return (view?.version.version ?? 0) < live.version ? live.draft : null;
}

/** The meeting's live notes, following Rust's `NotesRun` events. */
export function useLiveNotes(id: MeetingId): LiveNotes | null {
  const client = useQueryClient();
  const live = useQuery(liveNotesQuery(client, id));
  useIpcEvent(events.notesRun, (run) => {
    if (run.meetingId !== id) return;
    client.setQueryData<LiveNotes | null>(enhanceKeys.live(id), (previous) =>
      nextLive(previous, run.update),
    );
  });
  return live.data ?? null;
}

/** Forgets a finished or stopped run once its note has been read. */
export function useForgetRun(id: MeetingId): () => void {
  const client = useQueryClient();
  return () => {
    client.setQueryData<LiveNotes | null>(enhanceKeys.live(id), (previous) =>
      previous?.kind === 'writing' ? previous : null,
    );
  };
}

/** Stops writing the meeting's notes. */
export function useStopNotes(id: MeetingId) {
  return useMutation({ mutationFn: () => enhanceApi.stop(id) });
}
