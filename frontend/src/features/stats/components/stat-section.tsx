/**
 * A labelled block of the Stats page: a quiet 12/600 label over its
 * content, no card around it (ui.md §1 rule 4), and a resting line when
 * there is nothing to show.
 */
import type { ReactNode } from 'react';

import { cn } from '@/ui/cn';

/** Props of {@link StatSection}. */
export interface StatSectionProps {
  id: string;
  title: string;
  /** Muted text at the right of the label. */
  hint?: ReactNode;
  className?: string;
  children: ReactNode;
}

/** A section with a label. */
export function StatSection({ id, title, hint, className, children }: StatSectionProps) {
  return (
    <section aria-labelledby={id} className={cn('flex min-w-0 flex-col gap-3', className)}>
      <div className="flex items-baseline justify-between gap-3">
        <h2 id={id} className="text-xs font-semibold text-fg-muted">
          {title}
        </h2>
        {hint === undefined ? null : <p className="text-xs text-fg-subtle">{hint}</p>}
      </div>
      {children}
    </section>
  );
}

/** A section's resting line. */
export function RestingLine({ children }: { children: ReactNode }) {
  return <p className="py-2 text-sm text-fg-subtle">{children}</p>;
}
