/**
 * The card that drops in at the top centre of the screen, under the menu
 * bar: the meeting prompt and auto-stop's "Recording ends in…" notice. It
 * slides down on arrival and back up when answered (a fade only with Reduce
 * Motion), lifts a little under the pointer, and drains the time it has
 * left along a thin track at the bottom. An optional close button appears
 * on hover or keyboard focus. No logo: the title says what it is.
 */
import type { ReactNode } from 'react';

import { cn } from '@/ui/cn';
import { DrainBar } from '@/ui/drain-bar';
import { ICON_PROPS, X } from '@/ui/icons';

/** How long a card takes to slide away before its answer is sent. */
export { EDGE_EXIT_MS as TOP_EXIT_MS, useEdgeExit as useTopExit } from '@/ui/edge-card';

/** The entrance and the exit, as Tailwind classes. */
export const TOP_MOTION = {
  enter:
    'animate-in fade-in-0 duration-[420ms] ease-[cubic-bezier(0.2,0.9,0.25,1.15)] motion-safe:slide-in-from-top-6 motion-safe:zoom-in-[0.98]',
  exit: 'animate-out fill-mode-forwards fade-out-0 duration-200 ease-in motion-safe:slide-out-to-top-4',
} as const;

/** Props of {@link TopCard}. */
export interface TopCardProps {
  /** Names the card for assistive technology. */
  label: string;
  /** `status` for a notice that is announced; a plain region otherwise. */
  role?: 'status' | undefined;
  /** The card is sliding away. */
  leaving: boolean;
  /** Milliseconds the card has left, drained along its bottom. */
  drainMs: number;
  /** The drain's colour: the brand, or red for a recording about to stop. */
  tone?: 'brand' | 'danger' | undefined;
  /** Closes the card; without it there is no close button. */
  onClose?: (() => void) | undefined;
  /** The close button's accessible name. */
  closeLabel?: string | undefined;
  children: ReactNode;
}

/** The card: a title block and actions as children, the drain below. */
export function TopCard(props: TopCardProps) {
  const { onClose } = props;
  return (
    <section
      aria-label={props.label}
      role={props.role}
      className={cn(
        'group relative rounded-2xl bg-surface-raised p-3.5 text-fg shadow-popover',
        'transition-[translate,scale,box-shadow] duration-200 ease-out motion-safe:hover:translate-y-px motion-safe:hover:scale-[1.012]',
        props.leaving ? TOP_MOTION.exit : TOP_MOTION.enter,
      )}
    >
      {onClose === undefined ? null : (
        <button
          type="button"
          aria-label={props.closeLabel}
          onClick={onClose}
          className="absolute -top-2 -left-2 grid size-5 place-items-center rounded-full bg-surface-raised text-fg-muted opacity-0 shadow-popover transition duration-200 group-hover:opacity-100 hover:text-fg focus-visible:opacity-100 motion-safe:scale-75 motion-safe:group-hover:scale-100 motion-safe:focus-visible:scale-100"
        >
          <X {...ICON_PROPS} className="size-3" />
        </button>
      )}
      {props.children}
      <div className="relative mt-3 h-[3px] overflow-hidden rounded-full bg-bg-muted">
        <DrainBar
          durationMs={props.drainMs}
          className={cn('h-full', props.tone === 'danger' ? 'bg-danger' : 'bg-brand')}
        />
      </div>
    </section>
  );
}

/** The card's title and the line under it. */
export function TopCardHeading({ title, detail }: { title: string; detail: ReactNode }) {
  return (
    <div className="min-w-0">
      <h1 className="truncate text-sm font-semibold text-fg tabular-nums" title={title}>
        {title}
      </h1>
      <p className="flex min-w-0 gap-1 truncate text-xs text-fg-muted">{detail}</p>
    </div>
  );
}

/** Classes of the card's two buttons: the main answer and the quiet one. */
export const TOP_BUTTON = {
  primary:
    'flex h-8 flex-1 items-center justify-center rounded-lg bg-accent px-3 text-[13px] font-medium text-accent-fg transition hover:brightness-95 active:scale-[0.98] disabled:opacity-50',
  quiet:
    'flex h-8 flex-1 items-center justify-center rounded-lg bg-bg-muted px-3 text-[13px] font-medium text-fg transition hover:brightness-95 active:scale-[0.98] disabled:opacity-50',
} as const;
