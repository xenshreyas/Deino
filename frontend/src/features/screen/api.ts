/**
 * Typed calls to the `screen` plugin. Each resolves to the data or throws an
 * `IpcError`; components use the hooks in `queries.ts`.
 */
import { convertFileSrc } from '@tauri-apps/api/core';

import { commands, type MeetingId } from '@/lib/ipc/generated/screen';
import { unwrap } from '@/lib/ipc/result';

import type { CaptureSource, ScreenStatus, Snapshot, TargetChoice } from './types';

export const screenApi = {
  /** Whether this Mac can capture, the permission, and the current capture. */
  status: (): Promise<ScreenStatus> => unwrap(commands.status()),
  /** Opens System Settings → Privacy & Security → Screen Recording. */
  openScreenRecordingSettings: (): Promise<null> => unwrap(commands.openScreenRecordingSettings()),
  /** Stops the meeting's capture and keeps it off; true when one stopped. */
  turnOffForMeeting: (meetingId: MeetingId): Promise<boolean> =>
    unwrap(commands.turnOffForMeeting(meetingId)),
  /** The meeting's kept snapshots, in time order. */
  snapshots: (meetingId: MeetingId): Promise<Snapshot[]> => unwrap(commands.snapshots(meetingId)),
  /** The folder of the meeting's snapshot PNG files. */
  snapshotFolder: (meetingId: MeetingId): Promise<string> =>
    unwrap(commands.snapshotFolder(meetingId)),
  /** Deletes one snapshot and its picture; false when it was gone. */
  deleteSnapshot: ({ meetingId, id }: { meetingId: MeetingId; id: number }): Promise<boolean> =>
    unwrap(commands.deleteSnapshot(meetingId, id)),
  /** The picker's screens and windows. */
  captureChoices: (): Promise<TargetChoice[]> => unwrap(commands.captureChoices()),
  /** Captures `source` for the meeting from now on. */
  captureSource: (meetingId: MeetingId, source: CaptureSource): Promise<boolean> =>
    unwrap(commands.captureSource(meetingId, source)),
  /** Whether the meeting's capture shows the consent reminder. */
  consentReminder: (meetingId: MeetingId): Promise<boolean> =>
    unwrap(commands.consentReminder(meetingId)),
  /** "Don't show again". */
  stopConsentReminders: (): Promise<null> => unwrap(commands.stopConsentReminders()),
};

/** The asset URL of snapshot `fileName` in `folder` (inside the recordings). */
export function snapshotSrc(folder: string, fileName: string): string {
  return convertFileSrc(`${folder}/${fileName}`);
}
