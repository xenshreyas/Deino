/**
 * Typed calls to the `ask` plugin. Each resolves to the data or throws an
 * `IpcError`; components use the hooks in `queries.ts` and `hooks/`.
 * Answers stream over a per-request `Channel`; the call resolves with the
 * saved answer and its resolved citations.
 */
import { Channel } from '@tauri-apps/api/core';

import { commands } from '@/lib/ipc/generated/ask';
import type { MeetingId } from '@/lib/ipc/generated/meetings';
import type { RecipeId } from '@/lib/ipc/generated/recipes';
import { unwrap } from '@/lib/ipc/result';

import type { AskMessage, AskRequest, AskStreamEvent } from './types';

/** A channel that hands each piece of an answer to `onText`. */
function streamTo(onText: (delta: string) => void): Channel<AskStreamEvent> {
  const channel = new Channel<AskStreamEvent>();
  channel.onmessage = (event) => {
    onText(event.delta);
  };
  return channel;
}

export const askApi = {
  /** The meeting's thread, oldest first. */
  thread: (meeting: MeetingId): Promise<AskMessage[]> => unwrap(commands.thread(meeting)),
  /** Asks a question or runs a recipe; resolves with the saved answer. */
  ask: (
    meeting: MeetingId,
    request: AskRequest,
    onText: (delta: string) => void,
  ): Promise<AskMessage> => unwrap(commands.ask(meeting, request, streamTo(onText))),
  /** Runs a recipe on the meeting without saving anything. */
  preview: (
    meeting: MeetingId,
    recipe: RecipeId,
    onText: (delta: string) => void,
  ): Promise<AskMessage> => unwrap(commands.preview(meeting, recipe, streamTo(onText))),
  /** Empties the meeting's thread. */
  clear: (meeting: MeetingId): Promise<null> => unwrap(commands.clear(meeting)),
};
