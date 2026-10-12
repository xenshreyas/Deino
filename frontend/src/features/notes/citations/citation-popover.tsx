/**
 * The magnifier and its popover (NOT-03, Granola §3.2): beside an AI line
 * that cites the transcript, a quiet 🔍 that opens the cited excerpts, each
 * with its speaker and time, a Play button (TRN-03: plays the meeting's
 * recording from the excerpt's start, disabled without one) and "Show in
 * transcript". Keyboard: the magnifier is a button in
 * the tab order, the popover traps nothing and closes on Escape.
 */
import { useNavigate } from '@tanstack/react-router';

import type { MeetingId } from '@/features/meetings';
import { playerActions, usePlayerReady } from '@/features/player';
import { cn } from '@/ui/cn';
import { ICON_PROPS, Play, ScrollText, Search } from '@/ui/icons';
import { IconButton } from '@/ui/icon-button';
import { Popover, PopoverContent, PopoverTrigger } from '@/ui/popover';
import { Skeleton } from '@/ui/skeleton';

import { clockLabel, notesCopy } from '../copy';
import { useCitedSegments } from '../queries';
import type { CitationRef, CitedSegment, SegmentId } from '../types';

/** Props of {@link CitationPopover}. */
export interface CitationPopoverProps {
  meetingId: MeetingId;
  citations: CitationRef[];
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** Where the magnifier sits, in px from the top of the notes. */
  top: number;
}

/** The magnifier beside a cited line, and the excerpts it opens. */
export function CitationPopover({
  meetingId,
  citations,
  open,
  onOpenChange,
  top,
}: CitationPopoverProps) {
  return (
    <Popover open={open} onOpenChange={onOpenChange}>
      <PopoverTrigger
        aria-label={notesCopy.sourcesLabel}
        title={notesCopy.sourcesLabel}
        data-sources-trigger=""
        style={{ top }}
        className={cn(
          'absolute left-1 flex size-6 items-center justify-center rounded-sm text-fg-subtle',
          'transition-[color,background-color,opacity] duration-(--dur-fast) ease-out',
          'hover:bg-bg-muted hover:text-fg focus-visible:outline-2 focus-visible:outline-focus-ring',
          'data-[state=open]:bg-bg-muted data-[state=open]:text-fg',
        )}
      >
        <Search {...ICON_PROPS} size={14} />
      </PopoverTrigger>
      <PopoverContent align="start" side="bottom" className="w-[22rem] p-0">
        <h3 className="px-3 pt-3 pb-1 text-xs font-semibold text-fg-muted">
          {notesCopy.sourcesTitle}
        </h3>
        <CitedExcerpts meetingId={meetingId} citations={citations} enabled={open} />
      </PopoverContent>
    </Popover>
  );
}

/**
 * The excerpts, loading or gone: each with its speaker, time, Play and
 * "Show in transcript". Also what "Ask about this meeting" opens from an
 * answer's citation (AI-02-lite).
 */
export function CitedExcerpts({
  meetingId,
  citations,
  enabled,
}: {
  meetingId: MeetingId;
  citations: CitationRef[];
  enabled: boolean;
}) {
  const ids = citations.map((citation) => citation.segmentId as SegmentId);
  const cited = useCitedSegments(meetingId, ids, enabled);
  if (cited.data === undefined) {
    return (
      <div
        className="flex flex-col gap-2 px-3 pt-1 pb-3"
        role="status"
        aria-label={notesCopy.sourcesLoading}
      >
        <Skeleton className="h-3 w-24" />
        <Skeleton className="h-3 w-full" />
        <Skeleton className="h-3 w-3/4" />
      </div>
    );
  }
  if (cited.data.length === 0) {
    return <p className="px-3 pt-1 pb-3 text-sm text-fg-muted">{notesCopy.sourcesGone}</p>;
  }
  return (
    <ol className="max-h-80 overflow-y-auto pb-1">
      {cited.data.map((segment) => (
        <Excerpt key={segment.segmentId} meetingId={meetingId} segment={segment} />
      ))}
    </ol>
  );
}

/** One excerpt: who, when, what, and where to go from it. */
function Excerpt({ meetingId, segment }: { meetingId: MeetingId; segment: CitedSegment }) {
  const navigate = useNavigate();
  const playable = usePlayerReady(meetingId);
  const showInTranscript = () => {
    void navigate({
      to: '/meetings/$meetingId',
      params: { meetingId },
      search: { tab: 'transcript', segment: segment.segmentId },
    });
  };
  return (
    <li className="group/excerpt border-t border-border px-3 py-2.5 first:border-t-0">
      <div className="flex items-center gap-2">
        <span className="truncate text-xs font-semibold text-fg">{segment.speaker}</span>
        <span className="text-xs text-fg-muted tabular-nums">{clockLabel(segment.start)}</span>
        <span className="ml-auto flex items-center">
          <IconButton
            icon={Play}
            label={playable ? notesCopy.play : notesCopy.noRecording}
            disabled={!playable}
            onClick={() => {
              playerActions.playFrom(meetingId, segment.start);
            }}
          />
          <IconButton
            icon={ScrollText}
            label={notesCopy.showInTranscript}
            onClick={showInTranscript}
          />
        </span>
      </div>
      <p className="mt-1 text-sm text-pretty text-fg">{segment.text}</p>
    </li>
  );
}
