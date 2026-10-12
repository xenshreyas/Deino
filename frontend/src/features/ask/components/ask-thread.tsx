/**
 * The conversation: each question as a quiet bubble (with its `/recipe`),
 * each answer as text with citation chips, then the question on its way
 * with its answer streaming in, or why it failed.
 */
import type { MeetingId } from '@/features/meetings';
import { Button } from '@/ui/button';

import { streamingText } from '../answer-text';
import { askCopy } from '../copy';
import type { AskMessage, PendingAsk, RecipeLabel } from '../types';
import { AnswerBody } from './answer-body';

/** Props of {@link AskThread}. */
export interface AskThreadProps {
  meetingId: MeetingId;
  messages: readonly AskMessage[];
  pending: PendingAsk | null;
  error: Error | null;
  onRetry: (() => void) | undefined;
}

/** The messages in order, the pending one last. */
export function AskThread({ meetingId, messages, pending, error, onRetry }: AskThreadProps) {
  return (
    <ol className="grid gap-5" aria-label={askCopy.threadLabel}>
      {messages.map((message) => (
        <li key={message.id}>
          {message.role === 'user' ? (
            <Question
              text={message.parts.map((part) => part.text).join('')}
              recipe={message.recipe}
            />
          ) : (
            <AnswerBody meetingId={meetingId} parts={message.parts} />
          )}
        </li>
      ))}
      {pending === null ? null : (
        <li className="grid gap-5" aria-busy="true">
          <Question text={pending.question} recipe={pending.recipe} />
          <Streaming text={pending.streamed} />
        </li>
      )}
      {error === null || pending !== null ? null : <Failure error={error} onRetry={onRetry} />}
    </ol>
  );
}

/** A question, right-aligned on a muted fill. */
function Question({ text, recipe }: { text: string; recipe: RecipeLabel | null }) {
  return (
    <div className="flex justify-end">
      <p className="max-w-[85%] rounded-lg bg-bg-muted px-3 py-2 text-sm text-pretty whitespace-pre-wrap text-fg">
        {recipe === null ? null : (
          <span className="mr-1.5 font-mono text-xs text-fg-muted" title={recipe.name}>
            /{recipe.slug}
          </span>
        )}
        {text}
      </p>
    </div>
  );
}

/** The answer so far, its markers hidden until the citations resolve. */
function Streaming({ text }: { text: string }) {
  const shown = streamingText(text);
  if (shown === '') {
    return (
      <p role="status" className="animate-pulse text-sm text-fg-muted">
        {askCopy.thinking}
      </p>
    );
  }
  return <p className="text-sm leading-relaxed whitespace-pre-wrap text-fg">{shown}</p>;
}

function Failure({ error, onRetry }: { error: Error; onRetry: (() => void) | undefined }) {
  return (
    <li role="alert" className="flex items-center gap-2 text-sm text-danger">
      <span className="min-w-0 flex-1">{error.message}</span>
      {onRetry === undefined ? null : (
        <Button variant="ghost" size="sm" onClick={onRetry}>
          {askCopy.retry}
        </Button>
      )}
    </li>
  );
}
