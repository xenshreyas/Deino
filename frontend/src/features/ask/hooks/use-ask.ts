/**
 * Sending a question: the question shows at once, the answer streams in,
 * and once the saved answer is back the thread is refetched and the pending
 * copy goes away. A failure keeps the question so it can be retried.
 */
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { useState } from 'react';

import type { MeetingId } from '@/lib/ipc/generated/meetings';
import type { RecipeId } from '@/lib/ipc/generated/recipes';

import { askApi } from '../api';
import { askKeys } from '../queries';
import type { AskInput, AskMessage, PendingAsk } from '../types';

/** Appends `delta` to a pending answer. */
function grow(delta: string) {
  return (pending: PendingAsk | null): PendingAsk | null =>
    pending === null ? null : { ...pending, streamed: pending.streamed + delta };
}

/** Asks about `meeting`; see the module docs. */
export function useAsk(meeting: MeetingId) {
  const client = useQueryClient();
  const [pending, setPending] = useState<PendingAsk | null>(null);
  const mutation = useMutation({
    mutationFn: ({ request }: AskInput) =>
      askApi.ask(meeting, request, (delta) => {
        setPending(grow(delta));
      }),
    onMutate: ({ request, label }) => {
      setPending({ question: request.question, recipe: label, streamed: '' });
    },
    onSuccess: async () => {
      await client.invalidateQueries({ queryKey: askKeys.thread(meeting) });
      setPending(null);
    },
  });
  const failed = mutation.isError ? mutation.variables : undefined;
  return {
    send: (input: AskInput) => {
      mutation.mutate(input);
    },
    pending,
    busy: mutation.isPending,
    error: mutation.error,
    retry:
      failed === undefined
        ? undefined
        : () => {
            mutation.mutate(failed);
          },
  };
}

/** Runs a recipe on a meeting without saving it (Recipes → Preview). */
export function usePreview() {
  const [streamed, setStreamed] = useState('');
  const mutation = useMutation({
    mutationFn: ({ meeting, recipe }: { meeting: MeetingId; recipe: RecipeId }) =>
      askApi.preview(meeting, recipe, (delta) => {
        setStreamed((text) => text + delta);
      }),
    onMutate: () => {
      setStreamed('');
    },
  });
  const answer: AskMessage | undefined = mutation.data;
  return {
    run: mutation.mutate,
    streamed,
    answer,
    busy: mutation.isPending,
    error: mutation.error,
  };
}
