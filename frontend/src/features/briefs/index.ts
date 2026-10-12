/**
 * Pre-meeting briefs (NOT-08): two to four cited points before a meeting,
 * prepared by a background job from the user's own data. ORG-02's event
 * note and 1-minute notification drop in {@link EventBrief} or the
 * presentational {@link BriefCard}; meeting notes linked to an event show
 * {@link MeetingBrief}.
 */
export { briefsApi } from './api';
export { BriefCard, type BriefCardProps } from './components/brief-card';
export { BriefSettingsSection } from './components/brief-settings';
export { EventBrief, MeetingBrief } from './components/brief-slot';
export { CitationChip, citationLabel } from './components/citation-chip';
export { briefsCopy } from './copy';
export {
  briefKeys,
  useBriefFeedback,
  useEventBrief,
  useMarkBriefRead,
  useMeetingBrief,
} from './queries';
export type { Brief, BriefBullet, BriefCitation, BriefFeedback } from './types';
