/**
 * Empty state: every list and unbuilt screen has a calm resting state with one
 * clear action (ui.md §3 Empty state).
 */
import type { ReactNode } from 'react';

import { cn } from './cn';
import { type Icon, ICON_PROPS_LARGE } from './icons';

/** Props of {@link EmptyState}. */
export interface EmptyStateProps {
  icon: Icon;
  title: string;
  /** One or two short lines. */
  description?: string | undefined;
  /** The single action, usually a secondary button with a `Kbd`. */
  action?: ReactNode;
  className?: string;
}

/** Centred icon tile, title, description and one action. */
export function EmptyState({
  icon: IconComponent,
  title,
  description,
  action,
  className,
}: EmptyStateProps) {
  return (
    <div
      className={cn(
        'mx-auto flex max-w-[360px] flex-col items-center px-6 pt-[22vh] text-center',
        className,
      )}
    >
      <div className="flex size-10 items-center justify-center rounded-lg bg-bg-muted text-fg-muted">
        <IconComponent {...ICON_PROPS_LARGE} />
      </div>
      <h2 className="mt-3 text-sm font-semibold text-fg">{title}</h2>
      {description === undefined ? null : (
        <p className="mt-1 text-sm text-pretty text-fg-muted">{description}</p>
      )}
      {action === undefined ? null : <div className="mt-4">{action}</div>}
    </div>
  );
}
