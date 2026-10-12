/**
 * The preview's rows: each changed block, inline (removed words struck in
 * red, added words on the accent tint) or side by side (now | after).
 */
import type { ReactNode } from 'react';

import { cn } from '@/ui/cn';

import { editCopy } from './copy';
import type { DiffBlock, DiffLayout, DiffSegment, SegmentChange, TextKind } from './types';

/** How each kind of block starts, like the notes render it. */
const MARKERS: Partial<Record<TextKind, string>> = {
  bullet: '•',
  numbered: '·',
  todo: '☐',
  quote: '│',
};

/** The element each run is, so assistive tech hears insertions and deletions. */
const SEGMENT_TAGS = { same: 'span', removed: 'del', added: 'ins' } as const satisfies Record<
  SegmentChange,
  string
>;

const SEGMENT_CLASSES: Record<SegmentChange, string> = {
  same: '',
  removed: 'rounded-xs bg-danger-soft text-danger line-through decoration-1',
  added: 'rounded-xs bg-accent-soft text-fg no-underline',
};

/** Props of {@link DiffRows}. */
export interface DiffRowsProps {
  rows: readonly DiffBlock[];
  layout: DiffLayout;
}

/** Every changed block, with its section above when it changes. */
export function DiffRows({ rows, layout }: DiffRowsProps) {
  return (
    <ol className="flex flex-col gap-3" aria-label={editCopy.previewTitle}>
      {rows.map((row, index) => (
        <li key={index} className="flex flex-col gap-1">
          {row.section === null || row.section === rows[index - 1]?.section ? null : (
            <span className="text-xs font-medium text-fg-subtle">{row.section}</span>
          )}
          {layout === 'inline' ? <InlineRow row={row} /> : <SplitRow row={row} />}
        </li>
      ))}
    </ol>
  );
}

/** One row, kept, removed and added words together. */
function InlineRow({ row }: { row: DiffBlock }) {
  return (
    <BlockText kind={row.kind} badge={badge(row)}>
      <Segments segments={row.segments} />
    </BlockText>
  );
}

/** One row as two columns: the block now, and after the edit. */
function SplitRow({ row }: { row: DiffBlock }) {
  const before = row.segments.filter((segment) => segment.change !== 'added');
  const after = row.segments.filter((segment) => segment.change !== 'removed');
  return (
    <div className="grid grid-cols-2 gap-3">
      <Column
        label={editCopy.before}
        kind={row.kind}
        segments={row.change === 'added' ? [] : before}
      />
      <Column
        label={editCopy.after}
        kind={row.kind}
        segments={row.change === 'removed' ? [] : after}
      />
    </div>
  );
}

function Column(props: { label: string; kind: TextKind; segments: readonly DiffSegment[] }) {
  return (
    <div className="min-w-0 rounded-md bg-bg-subtle px-2.5 py-1.5" aria-label={props.label}>
      {props.segments.length === 0 ? (
        <span className="text-sm text-fg-subtle">—</span>
      ) : (
        <BlockText kind={props.kind} badge={null}>
          <Segments segments={props.segments} />
        </BlockText>
      )}
    </div>
  );
}

function BlockText(props: { kind: TextKind; badge: string | null; children: ReactNode }) {
  const marker = MARKERS[props.kind];
  return (
    <p
      className={cn(
        'flex gap-2 text-sm leading-6 text-fg',
        props.kind === 'heading' && 'font-semibold',
      )}
    >
      {marker === undefined ? null : (
        <span aria-hidden className="shrink-0 text-fg-subtle">
          {marker}
        </span>
      )}
      <span className="min-w-0 break-words">{props.children}</span>
      {props.badge === null ? null : (
        <span className="ml-auto shrink-0 self-start text-xs text-fg-subtle">{props.badge}</span>
      )}
    </p>
  );
}

function Segments({ segments }: { segments: readonly DiffSegment[] }) {
  return (
    <>
      {segments.map((segment, index) => {
        const Tag = SEGMENT_TAGS[segment.change];
        return (
          <Tag key={index} className={SEGMENT_CLASSES[segment.change]}>
            {segment.text}
          </Tag>
        );
      })}
    </>
  );
}

/** "Added" or "Removed" for a whole block. */
function badge(row: DiffBlock): string | null {
  if (row.change === 'added') return editCopy.added;
  if (row.change === 'removed') return editCopy.removed;
  return null;
}
