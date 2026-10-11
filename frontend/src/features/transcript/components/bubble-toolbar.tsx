/**
 * The floating toolbar on a hovered or focused bubble (ui.md Transcript
 * bubble, Hover): 28px high on the raised surface, beside the bubble's top
 * on the side facing the middle (bubbles leave a quarter of the column
 * free there), with the line's time ("· Edited" after an edit) and every
 * action the registry shows. When they do not all fit, the rarer ones go
 * into a ⋯ menu (`model/toolbar.ts`).
 */
import { formatElapsed } from '@/features/recording';
import { cn } from '@/ui/cn';
import { Ellipsis, ICON_PROPS } from '@/ui/icons';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuShortcut,
  DropdownMenuTrigger,
} from '@/ui/menu';
import { Hint } from '@/ui/tooltip';

import type { TranscriptAction, TranscriptActionContext } from '../actions';
import { transcriptCopy } from '../copy';
import { layoutToolbar } from '../model/toolbar';

/** Props of {@link BubbleToolbar}. */
export interface BubbleToolbarProps {
  context: TranscriptActionContext;
  actions: readonly TranscriptAction[];
  /** How many action buttons fit beside the bubble. */
  slots: number;
  /** In the tab order only on the focused line, so Tab does not walk every bubble. */
  tabbable: boolean;
}

/** The toolbar's small square buttons. */
const BUTTON_CLASS =
  'flex size-6 items-center justify-center rounded-sm text-fg-muted transition-colors duration-(--dur-fast) ease-out focus-visible:outline-2 focus-visible:outline-focus-ring enabled:hover:bg-bg-muted enabled:hover:text-fg disabled:opacity-40';

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
  const ActionIcon = action.icon;
  return (
    <Hint label={action.label} shortcut={action.shortcut?.display} side="top">
      <button
        type="button"
        aria-label={action.label}
        disabled={action.state(context) === 'disabled'}
        tabIndex={tabbable ? 0 : -1}
        onClick={() => void action.run(context)}
        className={BUTTON_CLASS}
      >
        <ActionIcon {...ICON_PROPS} size={14} />
      </button>
    </Hint>
  );
}

/** ⋯ and the actions that did not fit. */
function OverflowMenu({
  actions,
  context,
  tabbable,
}: {
  actions: readonly TranscriptAction[];
  context: TranscriptActionContext;
  tabbable: boolean;
}) {
  return (
    <DropdownMenu>
      <Hint label={transcriptCopy.moreActions} side="top">
        <DropdownMenuTrigger
          aria-label={transcriptCopy.moreActions}
          tabIndex={tabbable ? 0 : -1}
          className={BUTTON_CLASS}
        >
          <Ellipsis {...ICON_PROPS} size={14} />
        </DropdownMenuTrigger>
      </Hint>
      <DropdownMenuContent align={context.speaker.isMe ? 'end' : 'start'} className="min-w-[180px]">
        {actions.map((action) => {
          const ActionIcon = action.icon;
          return (
            <DropdownMenuItem
              key={action.id}
              destructive={action.id === 'delete'}
              disabled={action.state(context) === 'disabled'}
              onSelect={() => void action.run(context)}
            >
              <ActionIcon {...ICON_PROPS} />
              {action.label}
              {action.shortcut === undefined ? null : (
                <DropdownMenuShortcut>{action.shortcut.display}</DropdownMenuShortcut>
              )}
            </DropdownMenuItem>
          );
        })}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

/** The line's time, and "Edited" once the user changed it. */
function LineTime({ context }: { context: TranscriptActionContext }) {
  const { line } = context;
  return (
    <span className="px-1 text-xs whitespace-nowrap text-fg-subtle tabular-nums">
      {formatElapsed(line.start / 1000)}
      {line.editedAt === null ? null : ` · ${transcriptCopy.edited}`}
    </span>
  );
}

/** The toolbar; hidden until its bubble is hovered or holds focus. */
export function BubbleToolbar({ context, actions, slots, tabbable }: BubbleToolbarProps) {
  const shown = actions.filter((action) => action.state(context) !== 'hidden');
  const { inline, overflow } = layoutToolbar(shown, slots);
  return (
    <div
      role="toolbar"
      aria-label={transcriptCopy.bubbleActions}
      className={cn(
        'absolute top-0.5 z-(--z-sticky) flex h-7 items-center gap-0.5 rounded-md bg-surface-raised px-1 shadow-popover',
        'pointer-events-none opacity-0 transition-opacity duration-(--dur-fast) ease-out',
        'group-hover/bubble:pointer-events-auto group-hover/bubble:opacity-100',
        'group-focus-within/bubble:pointer-events-auto group-focus-within/bubble:opacity-100',
        'has-[[data-state=open]]:pointer-events-auto has-[[data-state=open]]:opacity-100',
        context.speaker.isMe ? 'right-full mr-1.5' : 'left-full ml-1.5',
      )}
    >
      <LineTime context={context} />
      {inline.map((action) => (
        <ActionButton key={action.id} action={action} context={context} tabbable={tabbable} />
      ))}
      {overflow.length === 0 ? null : (
        <OverflowMenu actions={overflow} context={context} tabbable={tabbable} />
      )}
    </div>
  );
}
