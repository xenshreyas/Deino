/**
 * The page's "About" block: the AI briefing the `update_entities` stage
 * writes after each meeting (in `--ai-text`, so AI words read as AI), when
 * it was written, and a quiet way to rewrite it now. Before the first
 * briefing it says when one will appear.
 */
import Markdown from 'react-markdown';
import { toast } from 'sonner';

import { Button } from '@/ui/button';
import { ICON_PROPS, RotateCcw } from '@/ui/icons';

import { entitiesCopy } from '../copy';
import { relativeDay } from '../format';
import type { SummaryOutcome } from '../types';
import { Section } from './entity-sections';

/** Props of {@link EntitySummary}. */
export interface EntitySummaryProps {
  summary: string | null;
  updatedAt: string | null;
  /** Rewrites the briefing; resolves to what happened. */
  refresh: () => Promise<SummaryOutcome>;
  refreshing: boolean;
}

/** What to tell the user when a refresh stored nothing. */
const SKIPPED: Partial<Record<SummaryOutcome, string>> = {
  no_model: entitiesCopy.aboutNoModel,
  no_content: entitiesCopy.aboutNoContent,
  empty_answer: entitiesCopy.aboutFailed,
};

/** Refresh, with the outcome told in a toast. */
function RefreshButton({
  refresh,
  refreshing,
}: Pick<EntitySummaryProps, 'refresh' | 'refreshing'>) {
  const run = () => {
    refresh().then(
      (outcome) => {
        const skipped = SKIPPED[outcome];
        if (skipped !== undefined) toast(skipped);
      },
      (error: unknown) => {
        const description = error instanceof Error ? error.message : undefined;
        toast.error(entitiesCopy.aboutFailed, { description });
      },
    );
  };
  return (
    <Button
      variant="ghost"
      size="sm"
      disabled={refreshing}
      onClick={run}
      className="-mr-2 h-6 text-xs"
    >
      <RotateCcw {...ICON_PROPS} size={14} className={refreshing ? 'animate-spin' : undefined} />
      {refreshing ? entitiesCopy.aboutRefreshing : entitiesCopy.aboutRefresh}
    </Button>
  );
}

/** The briefing, or the promise of one. */
export function EntitySummary({ summary, updatedAt, refresh, refreshing }: EntitySummaryProps) {
  const day = updatedAt === null ? null : relativeDay(updatedAt, new Date());
  // "Updated today", but "Updated Sep 28".
  const updated = day === 'Today' || day === 'Yesterday' ? day.toLowerCase() : day;
  return (
    <Section
      id="entity-about"
      title={entitiesCopy.about}
      trailing={<RefreshButton refresh={refresh} refreshing={refreshing} />}
    >
      {summary === null ? (
        <p className="px-3 py-1 text-sm text-pretty text-fg-subtle">{entitiesCopy.aboutEmpty}</p>
      ) : (
        <div className="px-3 text-sm leading-6 text-pretty text-ai-text [&_li]:my-1 [&_p]:my-2 [&_strong]:font-semibold [&_strong]:text-fg [&_ul]:list-disc [&_ul]:pl-5">
          <Markdown>{summary}</Markdown>
        </div>
      )}
      {updated === null ? null : (
        <p className="px-3 pt-1 text-xs text-fg-subtle">{entitiesCopy.aboutUpdated(updated)}</p>
      )}
    </Section>
  );
}
