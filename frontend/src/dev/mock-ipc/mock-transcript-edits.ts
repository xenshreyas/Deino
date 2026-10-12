/**
 * The transcript edits (TRN-02) over the mock `transcript_segments`, with
 * the Rust service's rules: a trimmed, non-blank edit keeps the first
 * `originalText`; delete and restore flip `isDeleted`; find and replace is
 * literal, ignores ASCII case unless asked, skips lines it would blank and
 * counts per meeting, newest meeting first. Every write is a `DbChanged`.
 */
import type * as Generated from '@/lib/ipc/generated/transcript';

import { failure, ok } from './command-result';
import { events } from './db';
import { mockDatabase, mockTranscripts } from './database';

let seq = 3_000_000;

/** When each meeting's transcript was last edited, for the notes' regenerate reasons. */
const editedAt = new Map<string, string>();

/** When the user last edited `meetingId`'s transcript in this tab, if ever. */
export function transcriptEditedAt(meetingId: string): string | null {
  return editedAt.get(meetingId) ?? null;
}

/** Records an edit of `meetings` and tells the UI the transcript changed. */
function changed(meetings: readonly string[]): void {
  const now = new Date().toISOString();
  for (const meeting of meetings) editedAt.set(meeting, now);
  seq += 1;
  void events.dbChanged.emit({ tables: ['transcript_segments'], seq });
}

const notFound = () =>
  failure<never>({ code: 'not_found', message: 'that line is no longer in the transcript' });

/** The stored segment with `id`, deleted or not. */
function find(id: Generated.SegmentId): Generated.Segment | undefined {
  return mockTranscripts.all().find((segment) => segment.id === id);
}

/** `transcript|edit`. */
export function editLine(id: Generated.SegmentId, raw: string) {
  const text = raw.trim();
  if (text === '')
    return failure<Generated.Segment>({
      code: 'invalid',
      message: 'a line cannot be empty; delete it instead',
    });
  const segment = find(id);
  if (segment === undefined || segment.isDeleted) return notFound();
  if (segment.text === text) return ok(segment);
  const edited = {
    ...segment,
    text,
    editedAt: new Date().toISOString(),
    originalText: segment.originalText ?? segment.text,
  };
  mockTranscripts.update(edited);
  changed([segment.meetingId]);
  return ok(edited);
}

/** `transcript|delete` and `transcript|restore`. */
export function setDeleted(id: Generated.SegmentId, isDeleted: boolean) {
  const segment = find(id);
  if (segment === undefined) return notFound();
  if (segment.isDeleted !== isDeleted) {
    mockTranscripts.update({ ...segment, isDeleted });
    changed([segment.meetingId]);
  }
  return ok(null);
}

/** Where `find` occurs in `text`, not overlapping, as the Rust `Pattern` matches. */
function occurrences(text: string, find: string, matchCase: boolean): number[] {
  const fold = (value: string) =>
    matchCase ? value : value.replace(/[A-Z]/g, (c) => c.toLowerCase());
  const haystack = fold(text);
  const needle = fold(find);
  const found: number[] = [];
  for (
    let at = haystack.indexOf(needle);
    at >= 0;
    at = haystack.indexOf(needle, at + needle.length)
  ) {
    found.push(at);
  }
  return found;
}

/** `text` with each occurrence replaced. */
function replaced(text: string, starts: readonly number[], length: number, by: string): string {
  let result = '';
  let cursor = 0;
  for (const start of starts) {
    result += text.slice(cursor, start) + by;
    cursor = start + length;
  }
  return result + text.slice(cursor);
}

/** The lines a request changes, with their new text, newest meeting first. */
function plan(request: Generated.ReplaceRequest, by: string) {
  const meetings = mockDatabase.meetings();
  const inScope = (segment: Generated.Segment) =>
    !segment.isDeleted &&
    meetings.some((meeting) => meeting.id === segment.meetingId) &&
    (request.scope.kind === 'all' || segment.meetingId === request.scope.meetingId);
  return meetings.flatMap((meeting) =>
    mockTranscripts
      .get(meeting.id)
      .segments.filter(inScope)
      .map((segment) => {
        const starts = occurrences(segment.text, request.find, request.matchCase);
        const text = replaced(segment.text, starts, request.find.length, by);
        return { segment, title: meeting.title, count: starts.length, text };
      })
      .filter(({ count, text }) => count > 0 && text.trim() !== ''),
  );
}

/** `transcript|preview`. */
export function previewReplace(request: Generated.ReplaceRequest) {
  if (request.find.trim() === '')
    return failure<Generated.ReplacePreview>({ code: 'invalid', message: 'type the text to find' });
  const preview: Generated.ReplacePreview = { matches: 0, segments: 0, meetings: [] };
  for (const { segment, title, count } of plan(request, '')) {
    preview.matches += count;
    preview.segments += 1;
    const meeting = preview.meetings.find((one) => one.meetingId === segment.meetingId);
    if (meeting === undefined)
      preview.meetings.push({ meetingId: segment.meetingId, title, matches: count });
    else meeting.matches += count;
  }
  return ok(preview);
}

/** `transcript|replace`. */
export function replaceAll(request: Generated.ReplaceRequest, by: string) {
  const preview = previewReplace(request);
  if (preview.status === 'error') return preview;
  const now = new Date().toISOString();
  for (const { segment, text } of plan(request, by)) {
    mockTranscripts.update({
      ...segment,
      text,
      editedAt: now,
      originalText: segment.originalText ?? segment.text,
    });
  }
  if (preview.data.segments > 0) changed(preview.data.meetings.map((one) => one.meetingId));
  const { matches, segments, meetings } = preview.data;
  return ok<Generated.ReplaceOutcome>({ matches, segments, meetings: meetings.length });
}
