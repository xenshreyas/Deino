/**
 * The transcript as lines: the saved segments (read with TanStack Query)
 * overlaid with the recorder's `live-segment` events that arrived since.
 *
 * An event is newer than the row it describes until the row is saved again,
 * so a live line replaces its saved row (keeping the row's speaker, which
 * events do not carry). A discarded line (echo of the call) leaves a
 * tombstone, so a stale saved row does not bring it back.
 */
import type { Segment, SegmentId, SegmentUpdate, TranscriptLine } from '../types';

/** A live line has no word timings until it is saved. */
const NO_WORDS: TranscriptLine['words'] = [];

/** Live lines by ID; `null` marks a discarded one. */
export type LiveLines = ReadonlyMap<SegmentId, TranscriptLine | null>;

/** The line of a saved segment. */
export function lineOfSegment(segment: Segment): TranscriptLine {
  return {
    id: segment.id,
    channel: segment.channel,
    speakerId: segment.speakerId,
    start: segment.start,
    end: segment.end,
    text: segment.text,
    words: segment.words,
    editedAt: segment.editedAt,
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
    channel: update.channel,
    speakerId: null,
    start: update.start,
    end: update.end,
    text: update.text,
    words: NO_WORDS,
    editedAt: null,
    open: update.state === 'open',
  });
  return next;
}

/** Spoken order: start time, then ID (the order Rust serves). */
function bySpokenOrder(a: TranscriptLine, b: TranscriptLine): number {
  return a.start - b.start || a.id.localeCompare(b.id);
}

/** Every visible line, in spoken order. Saved rows already come sorted. */
export function mergeLines(saved: readonly Segment[], live: LiveLines): TranscriptLine[] {
  const visible = saved.filter((segment) => !segment.isDeleted);
  if (live.size === 0) return visible.map(lineOfSegment);
  const lines = new Map<SegmentId, TranscriptLine>();
  for (const segment of visible) lines.set(segment.id, lineOfSegment(segment));
  for (const [id, line] of live) {
    if (line === null) lines.delete(id);
    else if (line.text.trim() !== '') {
      lines.set(id, { ...line, speakerId: lines.get(id)?.speakerId ?? null });
    }
  }
  return [...lines.values()].sort(bySpokenOrder);
}
