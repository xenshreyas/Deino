/**
 * "Ask about this meeting" (AI-02-lite, D13): the meeting page's Chat panel.
 * The thread fills the panel and stays pinned to its newest message
 * (a column-reverse scroller), the bar sits at the bottom. With no thread
 * yet, a short note and a few starting points: a question and two recipes.
 */
import type { MeetingId } from '@/features/meetings';
import { type Recipe, useRecipes } from '@/features/recipes';
import { Chip } from '@/ui/chip';
import { ICON_PROPS_LARGE, MessageCircle, Trash2 } from '@/ui/icons';
import { IconButton } from '@/ui/icon-button';
import { Skeleton } from '@/ui/skeleton';

import { askCopy } from '../copy';
import { useAsk } from '../hooks/use-ask';
import { useAskThread, useClearThread } from '../queries';
import type { AskInput } from '../types';
import { AskBar } from './ask-bar';
import { AskThread } from './ask-thread';

/** Props of {@link AskPanel}. */
export interface AskPanelProps {
  meetingId: MeetingId;
}

/** The thread and the bar. */
export function AskPanel({ meetingId }: AskPanelProps) {
  const thread = useAskThread(meetingId);
  const recipes = useRecipes().data ?? [];
  const ask = useAsk(meetingId);
  const clear = useClearThread(meetingId);
  const messages = thread.data ?? [];
  const empty = messages.length === 0 && ask.pending === null && ask.error === null;
  return (
    <div className="flex h-full flex-col">
      <div className="flex h-9 shrink-0 items-center justify-between px-4">
        <h3 className="text-xs font-semibold text-fg-muted">{askCopy.title}</h3>
        {messages.length === 0 ? null : (
          <IconButton
            icon={Trash2}
            label={askCopy.clear}
            disabled={clear.isPending || ask.busy}
            onClick={() => {
              clear.mutate();
            }}
          />
        )}
      </div>
      <div className="flex min-h-0 flex-1 flex-col-reverse overflow-y-auto px-4 pb-4">
        {thread.isPending ? <ThreadSkeleton /> : null}
        {!thread.isPending && empty ? <Starters recipes={recipes} onSend={ask.send} /> : null}
        {!thread.isPending && !empty ? (
          <AskThread
            meetingId={meetingId}
            messages={messages}
            pending={ask.pending}
            error={ask.error}
            onRetry={ask.retry}
          />
        ) : null}
      </div>
      <AskBar recipes={recipes} busy={ask.busy} onSend={ask.send} />
    </div>
  );
}

/** The empty thread: what this is, and three ways to start. */
function Starters({
  recipes,
  onSend,
}: {
  recipes: readonly Recipe[];
  onSend: (input: AskInput) => void;
}) {
  const suggested = askCopy.suggestedRecipes
    .map((slug) => recipes.find((recipe) => recipe.slug === slug))
    .filter((recipe): recipe is Recipe => recipe !== undefined);
  const askDecided = () => {
    onSend({ request: { question: askCopy.suggestions.decided, recipe: null }, label: null });
  };
  return (
    <div className="flex flex-col items-start gap-2 py-4">
      <div className="flex size-9 items-center justify-center rounded-lg bg-bg-muted text-fg-muted">
        <MessageCircle {...ICON_PROPS_LARGE} size={18} />
      </div>
      <p className="pt-1 text-sm text-pretty text-fg-muted">{askCopy.emptyBody}</p>
      <div className="flex flex-wrap gap-1.5 pt-1">
        <Chip onClick={askDecided}>{askCopy.suggestions.decided}</Chip>
        {suggested.map((recipe) => (
          <Chip
            key={recipe.id}
            onClick={() => {
              onSend({
                request: { question: '', recipe: recipe.id },
                label: { slug: recipe.slug, name: recipe.name },
              });
            }}
          >
            {recipe.name}
          </Chip>
        ))}
      </div>
    </div>
  );
}

function ThreadSkeleton() {
  return (
    <div className="grid gap-3 py-4">
      <Skeleton className="ml-auto h-8 w-2/3" />
      <Skeleton className="h-3 w-full" />
      <Skeleton className="h-3 w-4/5" />
    </div>
  );
}
