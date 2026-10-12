import type { ReactNode } from 'react';

import { type Icon, ICON_PROPS } from '@/ui/icons';

/** Props of {@link SetupRow}. */
export interface SetupRowProps {
  icon: Icon;
  /** The row's name, 14/600. */
  name: ReactNode;
  /** One line on what it is for, or its live state. */
  detail: ReactNode;
  /** The row's control: a button, a state, a countdown. */
  children: ReactNode;
}

/**
 * One line of a setup list: a muted icon, a name over one line of detail,
 * and the control on the right. Rows sit in a hairline-divided `<ul>`
 * ({@link SetupList}), never in cards (§3B).
 */
export function SetupRow({ icon: RowIcon, name, detail, children }: SetupRowProps) {
  return (
    <li className="flex min-h-16 items-center gap-4 py-3">
      <RowIcon {...ICON_PROPS} aria-hidden className="mt-0.5 shrink-0 self-start text-fg-muted" />
      <div className="flex min-w-0 flex-1 flex-col gap-0.5">
        <span className="flex items-center gap-2 text-sm font-semibold text-fg">{name}</span>
        <span className="text-sm text-fg-muted">{detail}</span>
      </div>
      <div className="flex shrink-0 items-center gap-3">{children}</div>
    </li>
  );
}

/** The hairline-divided list that holds {@link SetupRow}s. */
export function SetupList({ label, children }: { label?: string; children: ReactNode }) {
  return (
    <ul aria-label={label} className="flex flex-col divide-y divide-border border-y border-border">
      {children}
    </ul>
  );
}
