/**
 * The floating toolbar on a hovered or focused bubble (ui.md Transcript
 * bubble, Hover): 28px high on the raised surface, beside the bubble's top
 * on the side facing the middle (bubbles leave a quarter of the column
 * free there), with the line's time and every action the registry shows.
 */
import { formatElapsed } from '@/features/recording';
import { cn } from '@/ui/cn';
import { ICON_PROPS } from '@/ui/icons';
import { Hint } from '@/ui/tooltip';

import type { TranscriptAction, TranscriptActionContext } from '../actions';
import { transcriptCopy } from '../copy';

/** Props of {@link BubbleToolbar}. */
export interface BubbleToolbarProps {
  context: TranscriptActionContext;
  actions: readonly TranscriptAction[];
  /** In the tab order only on the focused line, so Tab does not walk every bubble. */
  tabbable: boolean;
}

/** One action button. */
function ActionButton({
  action,
  context,
  tabbable,
}: {
  action: TranscriptAction;
  context: TranscriptActionContext;
  tabbable: boolean;
}) {
  const state = action.state(context);
  const ActionIcon = action.icon;
  return (
    <Hint label={action.label} shortcut={action.shortcut?.display} side="top">
      <button
        type="button"
        aria-label={action.label}
        disabled={state === 'disabled'}
        tabIndex={tabbable ? 0 : -1}
        onClick={() => void action.run(context)}
        className="flex size-6 items-center justify-center rounded-sm text-fg-muted transition-colors duration-(--dur-fast) ease-out focus-visible:outline-2 focus-visible:outline-focus-ring enabled:hover:bg-bg-muted enabled:hover:text-fg disabled:opacity-40"
      >
        <ActionIcon {...ICON_PROPS} size={14} />
      </button>
    </Hint>
  );
}

/** The toolbar; hidden until its bubble is hovered or holds focus. */
export function BubbleToolbar({ context, actions, tabbable }: BubbleToolbarProps) {
  const shown = actions.filter((action) => action.state(context) !== 'hidden');
  return (
    <div
      role="toolbar"
      aria-label={transcriptCopy.bubbleActions}
      className={cn(
        'absolute top-0.5 z-(--z-sticky) flex h-7 items-center gap-0.5 rounded-md bg-surface-raised px-1 shadow-popover',
        'pointer-events-none opacity-0 transition-opacity duration-(--dur-fast) ease-out',
        'group-hover/bubble:pointer-events-auto group-hover/bubble:opacity-100',
        'group-focus-within/bubble:pointer-events-auto group-focus-within/bubble:opacity-100',
        context.speaker.isMe ? 'right-full mr-1.5' : 'left-full ml-1.5',
      )}
    >
      <span className="px-1 text-xs text-fg-subtle tabular-nums">
        {formatElapsed(context.line.start / 1000)}
      </span>
      {shown.map((action) => (
        <ActionButton key={action.id} action={action} context={context} tabbable={tabbable} />
      ))}
    </div>
  );
}
