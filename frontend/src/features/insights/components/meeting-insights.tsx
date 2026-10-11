/**
 * A meeting's Insights tab (ui.md §4.6 "Tabs: Notes, Transcript, Insights"):
 * each insight that ran here, its answers attributed to the moment they
 * were said, with "Run again" for stale or failed ones and "Run an
 * insight" for the rest. Reads like the notes: a document, not a dashboard.
 */
import { Link } from '@tanstack/react-router';
import { toast } from 'sonner';

import { Button, buttonVariants } from '@/ui/button';
import { EmptyState } from '@/ui/empty-state';
import { ChevronDown, ICON_PROPS, Lightbulb, Play } from '@/ui/icons';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/ui/menu';
import { SkeletonText } from '@/ui/skeleton';

import { insightsCopy } from '../copy';
import { useMeetingInsights, useRunInsight } from '../queries';
import type { InsightDef, InsightId, MeetingId, MeetingInsight } from '../types';
import { MeetingInsightSection } from './meeting-insight-section';

const copy = insightsCopy.meeting;

/** Props of {@link MeetingInsights}. */
export interface MeetingInsightsProps {
  meetingId: MeetingId;
}

/** Runs one insight on this meeting, toasting a failure to queue it. */
export function useRunHere(meetingId: MeetingId) {
  const run = useRunInsight();
  return {
    pending: run.isPending,
    run: (id: InsightId) => {
      run.mutate(
        { id, meetingIds: [meetingId] },
        { onError: (error) => toast.error(error.message) },
      );
    },
  };
}

/** The tab. */
export function MeetingInsights({ meetingId }: MeetingInsightsProps) {
  const insights = useMeetingInsights(meetingId);
  const here = useRunHere(meetingId);
  if (insights.data === undefined) {
    return (
      <div className="py-6">
        <SkeletonText lines={5} />
      </div>
    );
  }
  const { entries, others } = insights.data;
  if (entries.length === 0) return <Empty others={others} onRun={here.run} />;
  return (
    <div className="h-full overflow-y-auto pt-6 pb-16">
      <div className="mb-4 flex min-h-7 items-center justify-end gap-4">
        <h2 className="sr-only">{copy.heading}</h2>
        <RunMenu others={others} onRun={here.run} />
      </div>
      <div className="grid gap-10">
        {entries.map((entry: MeetingInsight) => (
          <MeetingInsightSection
            key={entry.def.id}
            entry={entry}
            meetingId={meetingId}
            onRerun={() => {
              here.run(entry.def.id);
            }}
          />
        ))}
      </div>
    </div>
  );
}

/** Nothing ran here yet: run one, or create one. */
function Empty({ others, onRun }: { others: InsightDef[]; onRun: (id: InsightId) => void }) {
  const action =
    others.length === 0 ? (
      <Link to="/insights" className={buttonVariants({ variant: 'secondary' })}>
        {copy.create}
      </Link>
    ) : (
      <RunMenu others={others} onRun={onRun} />
    );
  return (
    <EmptyState
      className="pt-[12vh]"
      icon={Lightbulb}
      title={copy.empty}
      description={copy.emptyBody}
      action={action}
    />
  );
}

/** "Run an insight": the insights that have not run here. */
function RunMenu({ others, onRun }: { others: InsightDef[]; onRun: (id: InsightId) => void }) {
  if (others.length === 0) return null;
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="secondary" size="sm">
          <Play {...ICON_PROPS} />
          {copy.runOne}
          <ChevronDown {...ICON_PROPS} className="text-fg-muted" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end">
        {others.map((def) => (
          <DropdownMenuItem
            key={def.id}
            onSelect={() => {
              onRun(def.id);
            }}
          >
            {def.name}
          </DropdownMenuItem>
        ))}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
