/**
 * Typed calls to the `player` plugin, the feature's only commands, and the
 * asset-protocol URL of a recording file (scoped to the recordings folder).
 */
import { convertFileSrc } from '@tauri-apps/api/core';

import { commands } from '@/lib/ipc/generated/player';
import { unwrap } from '@/lib/ipc/result';

import type { DownloadOutcome, MediaSource, MeetingId, MeetingMedia } from './types';

export const playerApi = {
  /** The meeting's playable recording files; none without a recording. */
  media: (meetingId: MeetingId): Promise<MeetingMedia> => unwrap(commands.media(meetingId)),
  /** Saves a copy of one file where the user chooses (Rust opens the dialog). */
  download: (meetingId: MeetingId, source: MediaSource): Promise<DownloadOutcome> =>
    unwrap(commands.download(meetingId, source)),
};

/** The URL the media element loads a recording file from. */
export function mediaUrl(path: string): string {
  return convertFileSrc(path);
}
