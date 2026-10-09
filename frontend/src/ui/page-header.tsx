/**
 * Page header: the title lives in the content, not in a toolbar
 * (ui.md §3 Page header).
 */
import type { ReactNode } from 'react';

import { cn } from './cn';

/** Props of {@link PageHeader}. */
export interface PageHeaderProps {
  title: ReactNode;
  /** One short line under the title. */
  description?: ReactNode;
  /** Right-aligned actions (md buttons). */
  actions?: ReactNode;
  className?: string;
}

/** 24/600 title, optional 14px muted subtitle, actions on the right. */
export function PageHeader({ title, description, actions, className }: PageHeaderProps) {
  return (
    <header className={cn('flex items-start justify-between gap-4 pb-6', className)}>
      <div className="min-w-0">
        <h1 className="text-xl font-semibold text-balance text-fg">{title}</h1>
        {description === undefined ? null : (
          <p className="mt-1 text-sm text-fg-muted">{description}</p>
        )}
      </div>
      {actions === undefined ? null : (
        <div className="flex shrink-0 items-center gap-2">{actions}</div>
      )}
    </header>
  );
}

/** Props of {@link Page}. */
export interface PageProps {
  children: ReactNode;
  /** `list` pages are 960px wide, `document` pages 720px (ui.md §3). */
  width?: 'list' | 'document';
  className?: string;
}

/** A centred, scrollable page body with the house gutters. */
export function Page({ children, width = 'list', className }: PageProps) {
  return (
    <div className="h-full overflow-y-auto">
      <div
        className={cn(
          'mx-auto w-full px-6 pt-8 pb-16 xl:px-8',
          width === 'list' ? 'max-w-[960px]' : 'max-w-[720px]',
          className,
        )}
      >
        {children}
      </div>
    </div>
  );
}
