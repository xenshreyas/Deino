/**
 * The live transcript as lines: the saved segments (read with TanStack
 * Query) overlaid with the `live-segment` events that arrived since.
 *
 * An event is newer than the row it describes until the row is saved again,
 * so a live line replaces its saved row. A discarded line (echo of the
 * call) leaves a tombstone, so a stale saved row does not bring it back.
 */
import type { Segment, SegmentId, SegmentUpdate, TranscriptLine } from './types';

/** Live lines by ID; `null` marks a discarded one. */
export type LiveLines = ReadonlyMap<SegmentId, TranscriptLine | null>;

/** The line of a saved segment. */
export function lineOfSegment(segment: Segment): TranscriptLine {
  return {
    id: segment.id,
    meetingId: segment.meetingId,
    channel: segment.channel,
    start: segment.start,
    text: segment.text,
    open: false,
  };
}

/** `live` with `update` applied. */
export function applyUpdate(live: LiveLines, update: SegmentUpdate): LiveLines {
  const next = new Map(live);
  if (update.state === 'discarded') {
    next.set(update.segmentId, null);
    return next;
  }
  next.set(update.segmentId, {
    id: update.segmentId,
    meetingId: update.meetingId,
    channel: update.channel,
    start: update.start,
    text: update.text,
    open: update.state === 'open',
  });
  return next;
}

/** Every visible line, in spoken order (start time, then ID). */
export function mergeLines(saved: readonly Segment[], live: LiveLines): TranscriptLine[] {
  const lines = new Map<SegmentId, TranscriptLine>();
  for (const segment of saved) {
    if (!segment.isDeleted) lines.set(segment.id, lineOfSegment(segment));
  }
  for (const [id, line] of live) {
    if (line === null) lines.delete(id);
    else if (line.text.trim() !== '') lines.set(id, line);
  }
  return [...lines.values()].sort((a, b) => a.start - b.start || a.id.localeCompare(b.id));
}
