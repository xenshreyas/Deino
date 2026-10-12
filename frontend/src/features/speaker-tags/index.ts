/**
 * Speaker tags (CAP-05): names from Zoom, Teams and Meet, read from the call
 * window through Accessibility while Deino records. The UI is one Settings
 * page; the names themselves reach the transcript through the speakers.
 */
export { speakerTagsApi } from './api';
export { SpeakerTagsSettings } from './components/speaker-tags-settings';
export { speakerTagsCopy } from './copy';
export { speakerTagKeys, speakerTagStatusQuery, useSpeakerTagStatus } from './queries';
export type { AppTagStatus, CallApp, TagState } from './types';
