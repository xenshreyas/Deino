/**
 * The action items feature (AI-05): the Action Items page, the meeting's
 * list at the top of its Notes tab, the "Today" digest (for Home, ORG-02),
 * the compact list in the meeting page's context rail ({@link CompactActionItems}),
 * and the notes adapter (`useActionItemTodos`) NOT-02's enhanced notes use
 * to render and tick the same items as todo blocks. Routes import their URL
 * contract from `./search`; other code imports from here only.
 */
export { actionItemsApi } from './api';
export { ActionItemsScreen, type ActionItemsScreenProps } from './components/action-items-screen';
export { CompactActionItems } from './components/compact-action-items';
export {
  MeetingActionItems,
  type MeetingActionItemsProps,
} from './components/meeting-action-items';
export { TodayDigest, type TodayDigestProps } from './components/today-digest';
export { copy as actionItemsCopy } from './copy';
export { type ActionItemTodo, meetingFilter, todoOf, useActionItemTodos } from './notes-sync';
export {
  actionItemKeys,
  actionItemListQuery,
  filterOptionsQuery,
  savedViewsQuery,
  todayQuery,
  useActionItems,
  useToday,
  useUpdateActionItem,
} from './queries';
export {
  type ActionItemFilter,
  type ActionItemId,
  type ActionItemView,
  asActionItemId,
  asViewId,
  MY_OPEN_ITEMS,
  type ViewChoice,
} from './types';
