/**
 * An answer's citation: a small numbered chip after the sentence it backs.
 * It opens NOT-03's excerpts (speaker, time, Play, Show in transcript) for
 * the segments it cites. Keyboard: a button in the tab order; Escape closes.
 */
import { useState } from 'react';

import type { MeetingId } from '@/features/meetings';
import { CitedExcerpts } from '@/features/notes';
import { cn } from '@/ui/cn';
import { Popover, PopoverContent, PopoverTrigger } from '@/ui/popover';

import { askCopy } from '../copy';
import type { AnswerCitation } from '../types';

/** Props of {@link CitationChip}. */
export interface CitationChipProps {
  meetingId: MeetingId;
  citations: readonly AnswerCitation[];
  /** Its number in the answer, from 1. */
  number: number;
}

/** The chip and its excerpts. */
export function CitationChip({ meetingId, citations, number }: CitationChipProps) {
  const [open, setOpen] = useState(false);
  const refs = citations.map((citation) => ({
    segmentId: citation.segmentId,
    startMs: citation.start,
  }));
  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger
        aria-label={askCopy.sources(number)}
        className={cn(
          'mx-0.5 inline-flex h-4 min-w-4 items-center justify-center rounded-sm px-1 align-[1px]',
          'bg-bg-muted text-[10px] font-medium text-fg-muted tabular-nums',
          'transition-colors duration-(--dur-fast) ease-out hover:bg-border hover:text-fg',
          'focus-visible:outline-2 focus-visible:outline-focus-ring',
          'data-[state=open]:bg-fg data-[state=open]:text-on-fg',
        )}
      >
        {number}
      </PopoverTrigger>
      <PopoverContent align="start" side="bottom" className="w-[22rem] p-0">
        <CitedExcerpts meetingId={meetingId} citations={refs} enabled={open} />
      </PopoverContent>
    </Popover>
  );
}
