/**
 * The top of a person or company page (ui.md §4.5): a 48px avatar, the
 * name at 24/600, one muted line of who they are, and the page's actions.
 */
import type { ReactNode } from 'react';

/** Props of {@link EntityHeader}. */
export interface EntityHeaderProps {
  avatar: ReactNode;
  name: string;
  /** The parts of the line under the name (title, company link, email), joined by dots. */
  meta: readonly ReactNode[];
  actions: ReactNode;
}

/** Avatar, name, meta line and actions. */
export function EntityHeader({ avatar, name, meta, actions }: EntityHeaderProps) {
  return (
    <header className="flex items-start gap-4">
      {avatar}
      <div className="flex min-w-0 flex-1 flex-col gap-1 pt-0.5">
        <h1 className="truncate text-xl font-semibold text-fg">{name}</h1>
        {meta.length === 0 ? null : (
          <p className="flex min-w-0 flex-wrap items-center gap-x-1.5 text-sm text-fg-muted">
            {meta.map((part, index) => (
              <span key={index} className="flex min-w-0 items-center gap-x-1.5">
                {index === 0 ? null : (
                  <span aria-hidden className="text-fg-subtle">
                    ·
                  </span>
                )}
                {part}
              </span>
            ))}
          </p>
        )}
      </div>
      <div className="flex shrink-0 items-center gap-1.5">{actions}</div>
    </header>
  );
}

/** Classes of an inline link in the meta line (a company, a website). */
export const META_LINK =
  'truncate text-fg-muted underline decoration-border-strong underline-offset-4 transition-colors duration-(--dur-fast) hover:text-fg hover:decoration-fg-muted';
