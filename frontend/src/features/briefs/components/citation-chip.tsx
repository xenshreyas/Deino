/**
 * One citation of a brief bullet: a small muted link to the meeting, the
 * moment, the person or the company it rests on. The invitation and items
 * without a meeting are plain labels.
 */
import { cn } from '@/ui/cn';

import { briefsCopy } from '../copy';
import type { BriefCitation } from '../types';

/** What a citation chip reads. */
export function citationLabel(citation: BriefCitation): string {
  switch (citation.kind) {
    case 'meeting':
      return citation.title;
    case 'moment':
    case 'action_item':
      return citation.label;
    case 'person':
    case 'company':
      return citation.name;
    case 'event':
      return briefsCopy.citation.event;
  }
}

/** Whether a citation leads somewhere. */
export function isNavigable(citation: BriefCitation): boolean {
  return citation.kind !== 'event' && citation.kind !== 'action_item';
}

/** Props of {@link CitationChip}. */
export interface CitationChipProps {
  citation: BriefCitation;
  onOpen: (citation: BriefCitation) => void;
}

/** A citation as a chip after the bullet. */
export function CitationChip({ citation, onOpen }: CitationChipProps) {
  const label = citationLabel(citation);
  const chip = 'inline-block max-w-48 truncate align-bottom rounded-sm px-1 text-xs text-fg-muted';
  if (!isNavigable(citation)) return <span className={chip}>{label}</span>;
  return (
    <button
      type="button"
      title={label}
      className={cn(
        chip,
        'transition-colors duration-150 ease-out hover:bg-bg-muted hover:text-fg focus-visible:ring-1 focus-visible:ring-focus-ring focus-visible:outline-none',
      )}
      onClick={() => {
        onOpen(citation);
      }}
    >
      {label}
    </button>
  );
}
