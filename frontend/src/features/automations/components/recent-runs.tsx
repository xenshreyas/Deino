/**
 * The builder's run log (AUT-01 "Run log"): the automation's latest runs,
 * newest first, each with its meeting, when it was asked for, its status,
 * and every step's result or error. `DbChanged` keeps it live while the
 * pipeline stage works.
 */
import { Link } from '@tanstack/react-router';

import { SkeletonText } from '@/ui/skeleton';

import { runCopy } from '../copy';
import { useRecentRuns, useStepCatalog } from '../queries';
import type { AutomationId, RecentRun } from '../types';
import { RunBadge, RunLog, shortDateTime } from './run-status';

/** Props of {@link RecentRuns}. */
export interface RecentRunsProps {
  automationId: AutomationId;
}

/** The run log, or a line saying how to start one. */
export function RecentRuns({ automationId }: RecentRunsProps) {
  const runs = useRecentRuns(automationId);
  const catalog = useStepCatalog().data ?? [];
  if (runs.data === undefined) return <SkeletonText lines={2} />;
  if (runs.data.length === 0) return <p className="text-sm text-fg-muted">{runCopy.noRuns}</p>;
  const label = (kind: string) => catalog.find((entry) => entry.kind === kind)?.label ?? kind;
  return (
    <ul aria-label={runCopy.recent} className="grid divide-y divide-border">
      {runs.data.map((entry) => (
        <RunRow key={entry.run.id} entry={entry} label={label} />
      ))}
    </ul>
  );
}

/** One run: its meeting, time and status, then its steps. */
function RunRow({ entry, label }: { entry: RecentRun; label: (kind: string) => string }) {
  const title = entry.meetingTitle === '' ? runCopy.untitledMeeting : entry.meetingTitle;
  return (
    <li className="grid gap-1.5 py-2.5 first:pt-0">
      <div className="flex min-w-0 items-center gap-3">
        <Link
          to="/meetings/$meetingId"
          params={{ meetingId: entry.run.meetingId }}
          aria-label={runCopy.openMeeting(title)}
          className="min-w-0 flex-1 truncate text-sm text-fg underline-offset-4 hover:underline"
        >
          {title}
        </Link>
        <time
          dateTime={entry.run.requestedAt}
          className="shrink-0 text-xs whitespace-nowrap text-fg-subtle tabular-nums"
        >
          {shortDateTime(entry.run.requestedAt)}
        </time>
        <RunBadge run={entry.run} />
      </div>
      <RunLog run={entry.run} label={(step) => label(step.kind)} />
    </li>
  );
}
