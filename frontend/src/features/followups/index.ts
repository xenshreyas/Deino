/**
 * Suggested follow-up emails (NOT-07): after a meeting with people from
 * outside the user's organisation, the pipeline may draft a follow-up.
 * Home lists ready drafts for three days through ORG-02's slot
 * ({@link useFollowUpSuggestions}); the meeting page shows
 * {@link MeetingFollowUp}, a block (in its context rail, or in Details on a
 * narrow window) whose "Open draft" opens the editor.
 */
export { followUpsApi } from './api';
export { FollowUpBlock, MeetingFollowUp } from './components/follow-up-block';
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
