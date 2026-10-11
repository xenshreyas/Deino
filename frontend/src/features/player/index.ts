/**
 * The meeting player (TRN-03): a calm docked bar under the meeting page
 * (play and pause, ±15 s, 0.75–2×, a scrubber painted with who spoke when,
 * the Mixed / Me / Them track, the screen recording when there is one,
 * Download through Rust's save dialog), playing the recording through the
 * asset protocol scoped to the recordings folder. Space and ←/→ drive it;
 * a linked moment (`?t=`) seeks it.
 *
 * | Export | Where |
 * |---|---|
 * | {@link MeetingPlayer} | the meeting page's player slot |
 * | {@link playerActions} | anything that seeks or plays: the transcript's "Play from here" and click to seek, the citations' Play |
 * | {@link usePlayerReady}, {@link usePlaybackValue}, {@link usePlaybackListener} | following playback: the transcript's current line and word |
 */
export { MeetingPlayer, type MeetingPlayerProps } from './components/meeting-player';
export { playerCopy } from './copy';
export { usePlaybackListener, usePlaybackValue, usePlayerReady } from './hooks/use-playback';
export { playerKeys, mediaQuery } from './queries';
export { playerActions } from './store';
export type { AudioSource, MediaSource, MeetingMedia, SpeakerBand } from './types';
