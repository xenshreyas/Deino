/**
 * Where a hit leads: its meeting (at the moment's time for a transcript
 * hit), a person's or company's page (ORG-04), a folder, or the action
 * items. People and companies still narrow a search when chosen as `@`
 * filters.
 */
import type { Hit } from './types';

/** What choosing a hit does. */
export type HitTarget =
  | { kind: 'meeting'; meetingId: string; atMs: number | null }
  | { kind: 'person'; personId: string }
  | { kind: 'company'; companyId: string }
  | { kind: 'folder'; folderId: string }
  | { kind: 'action-items' };

/** The target of `hit`. */
export function hitTarget(hit: Hit): HitTarget {
  switch (hit.kind) {
    case 'person':
      return { kind: 'person', personId: hit.refId };
    case 'company':
      return { kind: 'company', companyId: hit.refId };
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
