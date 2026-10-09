/**
 * Keyboard shortcut hint (`⌘K`), shown in tooltips, menus and empty states.
 */
import type { ReactNode } from 'react';

import { cn } from './cn';

/** Props of {@link Kbd}. */
export interface KbdProps {
  /** The keys, already in display form, for example `⌘K` or `[`. */
  children: ReactNode;
  className?: string;
}

/** A small key cap: 12px, `fg-subtle`, hairline `border-strong` (ui.md §2.5 radius-xs). */
export function Kbd({ children, className }: KbdProps) {
  return (
    <kbd
      className={cn(
        'inline-flex h-5 min-w-5 items-center justify-center rounded-xs border border-border-strong px-1',
        'font-sans text-xs text-fg-subtle tabular-nums',
        className,
      )}
    >
      {children}
    </kbd>
  );
}
