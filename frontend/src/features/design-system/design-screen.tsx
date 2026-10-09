/**
 * The dev-only `/design` page (PARITY_PLAN §3B): every token and primitive,
 * side by side in the light and dark themes, plus the `_example` reference
 * feature. UI cards check their work against it.
 */
import type { ReactNode } from 'react';

import { ExampleNotes } from '@/features/_example';
import { Page, PageHeader } from '@/ui/page-header';

import { ControlGallery } from './control-gallery';
import { FeedbackGallery } from './feedback-gallery';
import { TokenGallery } from './token-gallery';

/** Renders `children` once per theme, in two columns. */
function ThemeColumns({ children }: { children: ReactNode }) {
  return (
    <div className="grid gap-4 lg:grid-cols-2">
      {(['light', 'dark'] as const).map((theme) => (
        <div
          key={theme}
          data-theme={theme}
          className="rounded-xl border border-border bg-bg p-6 text-fg"
        >
          <p className="pb-4 text-xs font-semibold text-fg-muted capitalize">{theme}</p>
          {children}
        </div>
      ))}
    </div>
  );
}

/** One titled band of the page. */
function Band({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className="flex flex-col gap-3 pb-10">
      <h2 className="text-base font-semibold text-fg">{title}</h2>
      {children}
    </section>
  );
}

/** The design system page. */
export function DesignScreen() {
  return (
    <Page className="max-w-[1200px]">
      <PageHeader
        title="Design system"
        description="Tokens and primitives from docs/references/ui.md, in both themes."
      />
      <Band title="Tokens">
        <ThemeColumns>
          <TokenGallery />
        </ThemeColumns>
      </Band>
      <Band title="Controls">
        <ThemeColumns>
          <ControlGallery />
        </ThemeColumns>
      </Band>
      <Band title="Feedback and overlays">
        <ThemeColumns>
          <FeedbackGallery />
        </ThemeColumns>
      </Band>
      <Band title="Reference feature (_example)">
        <div className="rounded-xl border border-border">
          <ExampleNotes />
        </div>
      </Band>
    </Page>
  );
}
