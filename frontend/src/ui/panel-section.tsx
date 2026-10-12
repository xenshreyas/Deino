/**
 * A titled section of a side panel (the meeting's Details): a quiet 12px
 * label over its content, named for assistive tech by its title.
 */
import type { ReactNode } from 'react';

import { cn } from './cn';

/** Props of {@link PanelSection}. */
export interface PanelSectionProps {
  title: string;
  /** Beside the title, right-aligned (a count, an "Undo all"). */
  action?: ReactNode;
  className?: string;
  children: ReactNode;
}

/** The section. */
export function PanelSection({ title, action, className, children }: PanelSectionProps) {
  return (
    <section aria-label={title} className={cn('flex flex-col gap-1.5', className)}>
      <div className="flex min-h-6 items-center justify-between gap-2">
        <h3 className="text-xs text-fg-muted">{title}</h3>
        {action}
      </div>
      {children}
    </section>
  );
}
