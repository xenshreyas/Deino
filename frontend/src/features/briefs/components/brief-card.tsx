/**
 * `BriefCard`: a pre-meeting brief (NOT-08) as a calm, collapsible block at
 * the top of a note: two to four AI-written points, each with its
 * citations, and a thumbs down. Presentational: the brief and the handlers
 * come in as props, so ORG-02's event note and notification can drop it in.
 */
import { useState } from 'react';

import { cn } from '@/ui/cn';
import { ChevronDown, ChevronRight, ICON_PROPS, Sparkles, ThumbsDown } from '@/ui/icons';
import { IconButton } from '@/ui/icon-button';

import { briefsCopy } from '../copy';
import type { Brief, BriefBullet, BriefCitation } from '../types';
import { CitationChip } from './citation-chip';

/** Props of {@link BriefCard}. */
export interface BriefCardProps {
  brief: Brief;
  /** Open on first render: true until the user has read the brief once. */
  defaultExpanded: boolean;
  /** The thumbs down. */
  onNotUseful: () => void;
  /** A citation was clicked. */
  onOpenCitation: (citation: BriefCitation) => void;
}

/** One point and its citations. */
function BulletRow({
  bullet,
  onOpenCitation,
}: {
  bullet: BriefBullet;
  onOpenCitation: (citation: BriefCitation) => void;
}) {
  return (
    <li className="flex gap-2 text-sm leading-6 text-ai-text">
      <span aria-hidden className="mt-2.5 size-1 shrink-0 rounded-full bg-fg-subtle" />
      <span>
        {bullet.text}{' '}
        {bullet.citations.map((citation, index) => (
          <CitationChip key={index} citation={citation} onOpen={onOpenCitation} />
        ))}
      </span>
    </li>
  );
}

/** The header: label, point count when closed, the thumbs down and the toggle. */
function Header({
  brief,
  expanded,
  onToggle,
  onNotUseful,
}: {
  brief: Brief;
  expanded: boolean;
  onToggle: () => void;
  onNotUseful: () => void;
}) {
  const disliked = brief.feedback === 'not_useful';
  return (
    <div className="flex h-8 items-center gap-2">
      <button
        type="button"
        aria-expanded={expanded}
        aria-label={expanded ? briefsCopy.collapse : briefsCopy.expand}
        className="flex min-w-0 flex-1 items-center gap-2 text-left text-xs font-semibold text-fg-muted"
        onClick={onToggle}
      >
        {expanded ? <ChevronDown {...ICON_PROPS} /> : <ChevronRight {...ICON_PROPS} />}
        <Sparkles {...ICON_PROPS} />
        <span>{briefsCopy.title}</span>
        {expanded ? null : (
          <span className="font-normal">· {briefsCopy.points(brief.bullets.length)}</span>
        )}
      </button>
      <IconButton
        icon={ThumbsDown}
        label={disliked ? briefsCopy.thanks : briefsCopy.notUseful}
        disabled={disliked}
        aria-pressed={disliked}
        className={cn(disliked && 'text-fg')}
        onClick={onNotUseful}
      />
    </div>
  );
}

/** The brief, open until read, then collapsed to one line. */
export function BriefCard({ brief, defaultExpanded, onNotUseful, onOpenCitation }: BriefCardProps) {
  const [expanded, setExpanded] = useState(defaultExpanded);
  return (
    <section
      aria-label={briefsCopy.title}
      className="rounded-lg border border-border bg-bg-subtle px-3 py-1"
    >
      <Header
        brief={brief}
        expanded={expanded}
        onToggle={() => {
          setExpanded((open) => !open);
        }}
        onNotUseful={onNotUseful}
      />
      {expanded ? (
        <ul className="flex flex-col gap-1 pb-2 pl-6">
          {brief.bullets.map((bullet, index) => (
            <BulletRow key={index} bullet={bullet} onOpenCitation={onOpenCitation} />
          ))}
        </ul>
      ) : null}
    </section>
  );
}
