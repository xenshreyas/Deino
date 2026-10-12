/**
 * Mock of `@/lib/ipc/generated/ask`: each meeting's thread in memory, and
 * a canned answer streamed word by word whose citations name real fixture
 * segments, so `pnpm dev:mock` shows the chips and NOT-03's excerpts. The
 * sales discovery fixture starts with one question already answered.
 *
 * It imports no feature code, so the feature reaching it through its
 * generated bindings forms no cycle.
 */
import type * as Generated from '@/lib/ipc/generated/ask';

import { failure, ok } from './command-result';
import { fixtureUuid } from './fixture-ids';
import { latency } from './latency';
import { commands as recipes } from './recipes';

type Message = Generated.AskMessage;

/** The fixture whose thread starts with an answer. */
const DISCOVERY = 'meeting-fixture-sales-discovery';

function cite(meeting: string, index: number, startMs: number): Generated.AnswerCitation {
  return {
    segmentId: fixtureUuid(`segment/${meeting}/${String(index)}`) as Generated.SegmentId,
    start: startMs,
  };
}

function message(
  meeting: string,
  role: Generated.AskRole,
  parts: Generated.AnswerPart[],
  recipe: Generated.RecipeLabel | null = null,
): Message {
  return {
    id: crypto.randomUUID() as Generated.AskMessageId,
    meetingId: meeting as Generated.MeetingId,
    role,
    parts,
    recipe,
    createdAt: new Date().toISOString(),
  };
}

/** A canned answer for `meeting`, citing two of its segments. */
function answer(meeting: string): Generated.AnswerPart[] {
  return [
    {
      text: 'They run about 220 field technicians and 14 dispatchers across three regions',
      citations: [cite(meeting, 2, 9_000)],
    },
    {
      text: '. The DispatchPro licence ends on 31 January 2027, so a decision is needed before then',
      citations: [cite(meeting, 5, 41_000), cite(meeting, 6, 47_000)],
    },
    { text: '.', citations: [] },
  ];
}

const threads = new Map<string, Message[]>([
  [
    DISCOVERY,
    [
      message(DISCOVERY, 'user', [
        { text: 'How big is their field team, and when do they need to decide?', citations: [] },
      ]),
      message(DISCOVERY, 'assistant', answer(DISCOVERY)),
    ],
  ],
]);

/** Streams `parts` to the channel word by word, markers and all, like a model. */
async function stream(
  parts: Generated.AnswerPart[],
  onEvent: { onmessage: (event: Generated.AskStreamEvent) => void },
) {
  const text = parts
    .map((part) => `${part.text}${part.citations.length > 0 ? ' [s3]' : ''}`)
    .join('');
  for (const word of text.split(/(?<= )/)) {
    await new Promise((resolve) => setTimeout(resolve, 25));
    onEvent.onmessage({ kind: 'text', delta: word });
  }
}

/** How the thread labels a question that ran recipe `id`. */
async function recipeLabel(id: Generated.RecipeId | null): Promise<Generated.RecipeLabel | null> {
  if (id === null) return null;
  const listed = await recipes.list();
  const recipe = listed.status === 'ok' ? listed.data.find((r) => r.id === id) : undefined;
  return recipe === undefined ? null : { slug: recipe.slug, name: recipe.name };
}

export const commands = {
  async thread(meeting) {
    await latency();
    return ok(threads.get(meeting) ?? []);
  },
  async ask(meeting, request, onEvent) {
    await latency();
    if (request.question.trim() === '' && request.recipe === null) {
      return failure({ code: 'invalid', message: 'Type a question or choose a recipe.' });
    }
    const parts = answer(meeting);
    await stream(parts, onEvent);
    const label = await recipeLabel(request.recipe);
    const question = message(
      meeting,
      'user',
      request.question === '' ? [] : [{ text: request.question, citations: [] }],
      label,
    );
    const reply = message(meeting, 'assistant', parts);
    threads.set(meeting, [...(threads.get(meeting) ?? []), question, reply]);
    return ok(reply);
  },
  async preview(meeting, _recipe, onEvent) {
    await latency();
    const parts = answer(meeting);
    await stream(parts, onEvent);
    return ok(message(meeting, 'assistant', parts));
  },
  async clear(meeting) {
    await latency();
    threads.delete(meeting);
    return ok(null);
  },
} satisfies typeof Generated.commands;
