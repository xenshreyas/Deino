/**
 * Serializes fixture meetings the one way every tool writes them: keys in
 * schema order, two-space indents, one line per word timing (so a 90-minute
 * meeting stays reviewable in a diff).
 */

/** The meeting's keys in schema order, so diffs stay small. */
export function ordered(meeting) {
  const keys = ['id', 'title', 'kind', 'startedAt', 'durationMs', 'calendarEvent', 'companies',
    'attendees', 'speakers', 'segments', 'userNotes', 'groundTruth', 'audio'];
  return Object.fromEntries(keys.map((key) => [key, meeting[key] ?? null]));
}

/** JSON with two-space indents, but one line per word timing. */
export function formatMeeting(meeting) {
  const text = JSON.stringify(meeting, null, 2);
  return `${text.replace(
    /\{\n\s+"text": (".*?"),\n\s+"startMs": (\d+),\n\s+"endMs": (\d+)\n\s+\}/g,
    '{ "text": $1, "startMs": $2, "endMs": $3 }',
  )}\n`;
}
