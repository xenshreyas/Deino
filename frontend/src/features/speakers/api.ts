/**
 * Typed calls to the `speakers` plugin. Each resolves to the data or throws
 * an `IpcError`; components use the hooks in `queries.ts` instead.
 */
import { commands } from '@/lib/ipc/generated/speakers';
import { unwrap } from '@/lib/ipc/result';

import type {
  MeetingId,
  NameCandidate,
  Participant,
  PersonId,
  Rename,
  Renamed,
  SpeakerId,
  VoiceprintId,
  VoiceProfile,
} from './types';

export const speakersApi = {
  /** The meeting's speakers, most speech first. */
  participants: (meetingId: MeetingId): Promise<Participant[]> =>
    unwrap(commands.participants(meetingId)),
  /** Up to eight people to name a speaker after, attendees first. */
  candidates: (meetingId: MeetingId, query: string): Promise<NameCandidate[]> =>
    unwrap(commands.candidates(meetingId, query)),
  /** Names a speaker for one segment or all of them. */
  rename: (rename: Rename): Promise<Renamed> => unwrap(commands.rename(rename)),
  /** Folds speaker `from` into `into`. */
  merge: (meetingId: MeetingId, from: SpeakerId, into: SpeakerId): Promise<null> =>
    unwrap(commands.merge(meetingId, from, into)),
  /** "Yes, this is them." */
  confirmSuggestion: (meetingId: MeetingId, speakerId: SpeakerId): Promise<Renamed> =>
    unwrap(commands.confirmSuggestion(meetingId, speakerId)),
  /** "No, it is not." */
  dismissSuggestion: (speakerId: SpeakerId): Promise<null> =>
    unwrap(commands.dismissSuggestion(speakerId)),
  /** Everyone with a voice profile. */
  voiceProfiles: (): Promise<VoiceProfile[]> => unwrap(commands.voiceProfiles()),
  /** Deletes one voice sample. */
  deleteVoiceprint: (id: VoiceprintId): Promise<null> => unwrap(commands.deleteVoiceprint(id)),
  /** Deletes a person's whole voice profile. */
  forgetVoice: (personId: PersonId): Promise<null> => unwrap(commands.forgetVoice(personId)),
};
