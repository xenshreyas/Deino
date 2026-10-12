/**
 * Suggested follow-up emails (NOT-07): after a meeting with people from
 * outside the user's organisation, the pipeline may draft a follow-up.
 * Home lists ready drafts for three days through ORG-02's slot
 * ({@link useFollowUpSuggestions}); the meeting header shows
 * {@link MeetingFollowUp}, a chip whose "View draft" opens the editor.
 */
export { followUpsApi } from './api';
export { FollowUpChip, MeetingFollowUp } from './components/follow-up-chip';
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
