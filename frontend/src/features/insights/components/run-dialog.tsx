/**
 * "Run on past meetings": every finished meeting with a transcript, the
 * ones the insight's conditions match (and has not answered) pre-selected.
 * Running queues each meeting's `insights` stage in the background; the
 * page's progress line and table fill in as they finish.
 */
import { useState } from 'react';
import { toast } from 'sonner';

import { Button } from '@/ui/button';
import { cn } from '@/ui/cn';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/ui/dialog';
import { SkeletonText } from '@/ui/skeleton';

import { insightsCopy } from '../copy';
import { useRunCandidates, useRunInsight } from '../queries';
import type { InsightDef, MeetingId, RunCandidate } from '../types';

const copy = insightsCopy.run;

/** Props of {@link RunDialog}. */
export interface RunDialogProps {
  def: InsightDef;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

/** The dialog; its picker mounts once the candidates load. */
export function RunDialog({ def, open, onOpenChange }: RunDialogProps) {
  const candidates = useRunCandidates(def.id);
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-[560px]">
        <DialogHeader>
          <DialogTitle>{copy.title(def.name)}</DialogTitle>
          <DialogDescription>{copy.description}</DialogDescription>
        </DialogHeader>
        {candidates.data === undefined ? (
          <SkeletonText lines={5} />
        ) : (
          <Picker
            def={def}
            candidates={candidates.data}
            onDone={() => {
              onOpenChange(false);
            }}
          />
        )}
      </DialogContent>
    </Dialog>
  );
}

/** The meetings the dialog opens with selected. */
function preselected(candidates: readonly RunCandidate[]): ReadonlySet<MeetingId> {
  return new Set(
    candidates
      .filter((candidate) => candidate.matches && candidate.status !== 'done')
      .map((candidate) => candidate.meeting.id),
  );
}

/** The list, the quick selections and the run button. */
function Picker({
  def,
  candidates,
  onDone,
}: {
  def: InsightDef;
  candidates: RunCandidate[];
  onDone: () => void;
}) {
  const selection = useSelection(candidates);
  const { run, submit } = useSubmitRun(def, onDone);
  if (candidates.length === 0) return <p className="text-sm text-fg-muted">{copy.empty}</p>;
  return (
    <>
      <QuickSelect candidates={candidates} onSelect={selection.select} />
      <CandidateList label={copy.title(def.name)} candidates={candidates} selection={selection} />
      <DialogFooter>
        <Button type="button" variant="ghost" onClick={onDone}>
          {copy.cancel}
        </Button>
        <Button
          type="button"
          disabled={selection.chosen.size === 0 || run.isPending}
          onClick={() => {
            submit(selection.chosen);
          }}
        >
          {copy.submit(selection.chosen.size)}
        </Button>
      </DialogFooter>
    </>
  );
}

/** Queues the run on the chosen meetings, toasts the outcome, closes on success. */
function useSubmitRun(def: InsightDef, onDone: () => void) {
  const run = useRunInsight();
  const submit = (chosen: ReadonlySet<MeetingId>) => {
    const meetingIds = [...chosen];
    run.mutate(
      { id: def.id, meetingIds },
      {
        onSuccess: () => {
          toast.success(copy.started(meetingIds.length));
          onDone();
        },
        onError: (error) => toast.error(error.message),
      },
    );
  };
  return { run, submit };
}

/** Which meetings are ticked. */
interface Selection {
  chosen: ReadonlySet<MeetingId>;
  select: (ids: Iterable<MeetingId>) => void;
  toggle: (id: MeetingId) => void;
}

/** The ticked meetings, starting from the matching ones. */
function useSelection(candidates: readonly RunCandidate[]): Selection {
  const [chosen, setChosen] = useState(() => preselected(candidates));
  return {
    chosen,
    select: (ids) => {
      setChosen(new Set(ids));
    },
    toggle: (id) => {
      const next = new Set(chosen);
      if (!next.delete(id)) next.add(id);
      setChosen(next);
    },
  };
}

/** Select matching, all, or none. */
function QuickSelect({
  candidates,
  onSelect,
}: {
  candidates: readonly RunCandidate[];
  onSelect: (ids: Iterable<MeetingId>) => void;
}) {
  const choices = [
    { label: copy.matching, ids: () => preselected(candidates) },
    { label: copy.all, ids: () => candidates.map((candidate) => candidate.meeting.id) },
    { label: copy.none, ids: () => [] },
  ];
  return (
    <div className="flex gap-1">
      {choices.map((choice) => (
        <Button
          key={choice.label}
          type="button"
          variant="ghost"
          size="sm"
          onClick={() => {
            onSelect(choice.ids());
          }}
        >
          {choice.label}
        </Button>
      ))}
    </div>
  );
}

/** The meetings, newest first. */
function CandidateList({
  label,
  candidates,
  selection,
}: {
  label: string;
  candidates: readonly RunCandidate[];
  selection: Selection;
}) {
  return (
    <ul aria-label={label} className="-mx-2 max-h-[360px] overflow-y-auto">
      {candidates.map((candidate) => (
        <CandidateRow
          key={candidate.meeting.id}
          candidate={candidate}
          checked={selection.chosen.has(candidate.meeting.id)}
          onToggle={() => {
            selection.toggle(candidate.meeting.id);
          }}
        />
      ))}
    </ul>
  );
}

const STATUS_LABEL = { done: copy.done, pending: copy.pending, failed: copy.failed } as const;

/** A meeting: tick, title, date, and where the insight stands on it. */
function CandidateRow({
  candidate,
  checked,
  onToggle,
}: {
  candidate: RunCandidate;
  checked: boolean;
  onToggle: () => void;
}) {
  const { meeting, status, matches } = candidate;
  const note = status === null ? (matches ? copy.matches : null) : STATUS_LABEL[status];
  return (
    <li>
      <label className="flex h-10 cursor-default items-center gap-3 rounded-md px-2 transition-colors duration-(--dur-fast) ease-out hover:bg-bg-muted">
        <input
          type="checkbox"
          checked={checked}
          onChange={onToggle}
          className="size-4 shrink-0 accent-accent"
        />
        <span className="min-w-0 flex-1 truncate text-sm text-fg">{meeting.title}</span>
        {note === null ? null : (
          <span className={cn('text-xs', status === 'failed' ? 'text-danger' : 'text-fg-muted')}>
            {note}
          </span>
        )}
        <span className="w-16 shrink-0 text-right text-xs text-fg-subtle tabular-nums">
          {new Date(meeting.startedAt).toLocaleDateString('en-US', {
            month: 'short',
            day: 'numeric',
          })}
        </span>
      </label>
    </li>
  );
}
