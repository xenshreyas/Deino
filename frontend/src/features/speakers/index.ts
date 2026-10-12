/**
 * The speakers feature (CAP-04): the meeting's participants, in Details (names,
 * talk time, "Is this …?", rename and merge), the "Who is …?" popover that
 * TRN-01's transcript chips open for one segment, and the voice profiles
 * that Settings → People lists today and ORG-04's person page can embed.
 * Other code imports from here only.
 */
export { speakersApi } from './api';
export { AssignSpeaker, type AssignSpeakerProps } from './components/assign-speaker';
export { ParticipantsPanel, type ParticipantsPanelProps } from './components/participants-panel';
export { PeopleSettings } from './components/people-settings';
export { SpeakerAvatar, type SpeakerAvatarProps } from './components/speaker-avatar';
export { VoiceProfiles } from './components/voice-profiles';
export { speakersCopy } from './copy';
export { displayName, speakerTone, talkTime } from './format';
export {
  participantsQuery,
  speakerKeys,
  useParticipants,
  useRenameSpeaker,
  useVoiceProfiles,
} from './queries';
export type {
  NameCandidate,
  Participant,
  Rename,
  Renamed,
  RenameScope,
  SpeakerName,
  VoiceProfile,
} from './types';
