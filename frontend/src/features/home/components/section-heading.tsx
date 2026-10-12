/** A Home section's heading: 12/600 muted capitals (ui.md §4.1). */
import type { ReactNode } from 'react';

/** Props of {@link SectionHeading}. */
export interface SectionHeadingProps {
  id: string;
  children: ReactNode;
  /** A trailing link or button ("View all"). */
  action?: ReactNode;
}

/** The heading row of a Home section. */
export function SectionHeading({ id, children, action }: SectionHeadingProps) {
  return (
    <div className="flex h-8 items-center justify-between px-3">
      <h2 id={id} className="text-xs font-semibold tracking-wide text-fg-muted uppercase">
        {children}
      </h2>
      {action}
    </div>
  );
}
