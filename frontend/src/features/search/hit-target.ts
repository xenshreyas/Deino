/**
 * Where a hit leads: its meeting (at the moment's time for a transcript
 * hit), a filter (people and companies narrow the search to their
 * meetings), a folder, or the action items.
 */
import type { FilterChip, Hit } from './types';

/** What choosing a hit does. */
export type HitTarget =
  | { kind: 'meeting'; meetingId: string; atMs: number | null }
  | { kind: 'filter'; chip: FilterChip }
  | { kind: 'folder'; folderId: string }
  | { kind: 'action-items' };

/** The target of `hit`. */
export function hitTarget(hit: Hit): HitTarget {
  switch (hit.kind) {
    case 'person':
    case 'company':
      return { kind: 'filter', chip: { kind: hit.kind, id: hit.refId, label: hit.label.text } };
    case 'folder':
      return { kind: 'folder', folderId: hit.refId };
    case 'segment':
      return meetingTarget(hit, hit.tsMs);
    case 'meeting':
    case 'note':
    case 'summary':
    case 'action_item':
      return meetingTarget(hit, null);
  }
}

function meetingTarget(hit: Hit, atMs: number | null): HitTarget {
  return hit.meetingId === null
    ? { kind: 'action-items' }
    : { kind: 'meeting', meetingId: hit.meetingId, atMs };
}

/** A stable key for a hit within its section. */
export function hitKey(hit: Hit): string {
  return `${hit.kind}:${hit.refId}:${String(hit.tsMs ?? '')}`;
}
