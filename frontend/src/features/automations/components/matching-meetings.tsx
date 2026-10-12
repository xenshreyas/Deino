/**
 * The "Matching meetings" tab: every finished meeting the automation's
 * conditions match, whether it ran there, and "Run on selected".
 */
import { useState } from 'react';

import { Button } from '@/ui/button';
import { Play, ICON_PROPS } from '@/ui/icons';
import { SkeletonText } from '@/ui/skeleton';

import { runCopy } from '../copy';
import { useMatchingMeetings, useRunAutomation } from '../queries';
import type { AutomationId, MatchingMeeting, MeetingId } from '../types';
import { RunBadge, shortDate } from './run-status';

/** Props of {@link MatchingMeetings}. */
export interface MatchingMeetingsProps {
  automationId: AutomationId;
}

/** The tab. */
export function MatchingMeetings({ automationId }: MatchingMeetingsProps) {
  const matching = useMatchingMeetings(automationId);
  const run = useRunAutomation();
  const { selected, setSelected, toggle } = useSelection();
  if (matching.data === undefined) return <SkeletonText lines={4} />;
  if (matching.data.length === 0) {
    return <p className="text-sm text-fg-muted">{runCopy.noMatches}</p>;
  }
  const all = matching.data.map((entry) => entry.meeting.id);
  return (
    <div className="grid gap-3">
      <SelectionBar
        selected={selected.length}
        total={all.length}
        busy={run.isPending}
        onSelectAll={(on) => {
          setSelected(on ? all : []);
        }}
        onRun={() => {
          run.mutate({ id: automationId, meetingIds: selected });
          setSelected([]);
        }}
      />
      <ul className="grid divide-y divide-border">
        {matching.data.map((entry) => (
          <MatchingRow
            key={entry.meeting.id}
            entry={entry}
            checked={selected.includes(entry.meeting.id)}
            onToggle={toggle}
          />
        ))}
      </ul>
    </div>
  );
}

/** The picked meetings. */
function useSelection() {
  const [selected, setSelected] = useState<MeetingId[]>([]);
  const toggle = (id: MeetingId) => {
    setSelected((current) =>
      current.includes(id) ? current.filter((other) => other !== id) : [...current, id],
    );
  };
  return { selected, setSelected, toggle };
}

/** "Select all" and "Run on N selected". */
function SelectionBar(props: {
  selected: number;
  total: number;
  busy: boolean;
  onSelectAll: (on: boolean) => void;
  onRun: () => void;
}) {
  return (
    <div className="flex flex-wrap items-center justify-between gap-3">
      <label className="flex items-center gap-3 text-sm text-fg-muted">
        <input
          type="checkbox"
          className="size-4 accent-accent"
          checked={props.selected === props.total}
          onChange={(event) => {
            props.onSelectAll(event.target.checked);
          }}
        />
        {runCopy.selectAll}
      </label>
      <Button size="sm" disabled={props.selected === 0 || props.busy} onClick={props.onRun}>
        <Play {...ICON_PROPS} />
        {runCopy.runSelected(props.selected)}
      </Button>
    </div>
  );
}

/** One meeting: pick it, its title and date, its run. */
function MatchingRow({
  entry,
  checked,
  onToggle,
}: {
  entry: MatchingMeeting;
  checked: boolean;
  onToggle: (id: MeetingId) => void;
}) {
  return (
    <li className="flex min-h-10 items-center gap-3 py-2">
      <input
        type="checkbox"
        className="size-4 shrink-0 accent-accent"
        aria-label={entry.meeting.title}
        checked={checked}
        onChange={() => {
          onToggle(entry.meeting.id);
        }}
      />
      <span className="min-w-0 flex-1 truncate text-sm text-fg">{entry.meeting.title}</span>
      <time
        dateTime={entry.meeting.startedAt}
        className="shrink-0 text-xs whitespace-nowrap text-fg-subtle tabular-nums"
      >
        {shortDate(entry.meeting.startedAt)}
      </time>
      <RunBadge run={entry.run} />
    </li>
  );
}
