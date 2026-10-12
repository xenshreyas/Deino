/**
 * Typed calls to the `screen` plugin. Each resolves to the data or throws an
 * `IpcError`; components use the hooks in `queries.ts`.
 */
import { commands, type MeetingId } from '@/lib/ipc/generated/screen';
import { unwrap } from '@/lib/ipc/result';

import type { ScreenStatus, Snapshot } from './types';

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
};
