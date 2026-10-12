/**
 * Clips and moments (TRN-04): a link to a time of a meeting
 * (`deino://meeting/<id>?t=<ms>`), and clips: a named range of the
 * recording cut into `<recording folder>/clips/<name>.m4a` (`.mp4` with a
 * screen recording), listed in the meeting's Details panel (play through
 * the player, rename, Finder, delete) and in ⌘K.
 *
 * | Export | Where |
 * |---|---|
 * | {@link MeetingClips} | the meeting's Details panel, when it has clips |
 * | {@link ClipDialog} | the meeting page, once: names a clip being made |
 * | {@link ClipsPalette} | the ⌘K palette's providers |
 * | {@link clipSpan}, {@link copySpanLink} | the transcript's Clip action and selection menu |
 * | {@link momentLink} | anything that links to a moment |
 */
export { clipSpan, copySpanLink, type TimedSpan } from './actions';
export { ClipDialog } from './components/clip-dialog';
export { ClipsPalette } from './components/clips-palette';
export { MeetingClips, type MeetingClipsProps } from './components/meeting-clips';
export { clipsCopy } from './copy';
export { copyMomentLink, momentLink } from './moment-link';
export { clipKeys } from './queries';
export type { Clip } from './types';
