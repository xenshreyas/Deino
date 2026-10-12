/**
 * Suggested follow-up emails (NOT-07): after a meeting with people from
 * outside the user's organisation, the pipeline may draft a follow-up.
 * Home lists ready drafts for three days through ORG-02's slot
 * ({@link useFollowUpSuggestions}); the meeting's Notes tab shows
 * {@link MeetingFollowUp}, whose "View draft" opens the editor.
 */
export { followUpsApi } from './api';
export { FollowUpBanner, MeetingFollowUp } from './components/follow-up-banner';
export { FollowUpEditor } from './components/follow-up-editor';
export { followUpsCopy } from './copy';
export { placeholderCount, textRuns } from './draft-text';
export {
  followUpKeys,
  useFollowUpSuggestions,
  useMeetingFollowUp,
  useSuggestedFollowUps,
} from './queries';
export type { FollowUpDraft, FollowUpEdit, FollowUpSummary } from './types';
