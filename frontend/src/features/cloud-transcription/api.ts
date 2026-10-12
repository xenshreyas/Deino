/**
 * Typed calls to the `cloud-transcription` plugin (CAP-14). Each resolves
 * to the data or throws an `IpcError`; components use the hooks in
 * `queries.ts`. There is no way to read the key back.
 */
import { commands } from '@/lib/ipc/generated/cloud-transcription';
import { unwrap } from '@/lib/ipc/result';

import type { MeetingId, SecretStatus, StageStatus } from './types';

export const cloudTranscriptionApi = {
  /** Whether an ElevenLabs key is saved; never the key. */
  keyStatus: (): Promise<SecretStatus> => unwrap(commands.keyStatus()),
  /** Saves the key in the Keychain and allows ElevenLabs on the Network page. */
  saveKey: (key: string): Promise<null> => unwrap(commands.saveKey(key)),
  /** Asks ElevenLabs whether it knows the saved key. */
  checkKey: (): Promise<null> => unwrap(commands.checkKey()),
  /** Forgets the key and blocks ElevenLabs again. */
  forgetKey: (): Promise<null> => unwrap(commands.forgetKey()),
  /** Transcribes the meeting again with ElevenLabs, then every later step. */
  retranscribe: (meetingId: MeetingId): Promise<StageStatus[]> =>
    unwrap(commands.retranscribe(meetingId)),
  /** Whether the transcript made on this Mac was kept. */
  hasOriginal: (meetingId: MeetingId): Promise<boolean> => unwrap(commands.hasOriginal(meetingId)),
  /** Puts the kept transcript back. */
  restoreOriginal: (meetingId: MeetingId): Promise<boolean> =>
    unwrap(commands.restoreOriginal(meetingId)),
};
