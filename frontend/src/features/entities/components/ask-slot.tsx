/**
 * Where chat scoped to one person or company (AI-02) will live: a calm,
 * disabled "Ask about …" bar with a WIP badge (owner decision D13: the Ask
 * Deino agent is deprioritised, not removed). AI-02 replaces it through
 * `slots.ts`.
 */
import { ICON_PROPS, MessageCircle } from '@/ui/icons';
import { Hint } from '@/ui/tooltip';

import { entitiesCopy } from '../copy';
import type { AskSlotProps } from '../slots';

/** The resting ask bar. */
export function AskSlotPlaceholder({ name }: AskSlotProps) {
  return (
    <Hint label={entitiesCopy.askSoon}>
      <div
        tabIndex={0}
        role="group"
        aria-disabled
        aria-label={entitiesCopy.ask(name)}
        className="flex h-9 w-full cursor-default items-center gap-2.5 rounded-full border border-border bg-surface px-3.5 text-sm text-fg-subtle select-none focus-visible:outline-2 focus-visible:outline-focus-ring"
      >
        <MessageCircle {...ICON_PROPS} />
        <span className="min-w-0 flex-1 truncate">{entitiesCopy.ask(name)}</span>
        <span className="rounded-full bg-bg-muted px-1.5 py-px text-[10px] font-semibold tracking-wide text-fg-muted">
          {entitiesCopy.wip}
        </span>
      </div>
    </Hint>
  );
}
