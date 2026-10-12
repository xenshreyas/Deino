/** Every string Home shows (ORG-02). */
import type { RecordReason } from '@/lib/ipc/generated/calendar';

/** The greeting for the hour of the day. */
function greeting(hour: number): string {
  if (hour < 12) return 'Good morning';
  if (hour < 18) return 'Good afternoon';
  return 'Good evening';
}

/** Why an event will or will not record, for the toggle's tooltip. */
const reasons: Record<RecordReason, string> = {
  eventChoice: 'You chose this for this event',
  seriesChoice: 'You chose this for every occurrence',
  declined: 'You declined it',
  notAMeeting: 'Nobody else is invited',
  audienceOff: 'Auto-record is off for meetings like this',
  organizedByOthers: 'Someone else organised it',
  notAccepted: 'You have not accepted it',
  rules: 'Matches your auto-record settings',
};

export const homeCopy = {
  greeting,
  comingUp: 'Coming up',
  recentNotes: 'Recent notes',
  viewAll: 'View all',
  noNotes: 'No notes yet',
  noNotesBody: 'Record a meeting and its notes land here.',
  actionItems: 'Open action items',
  followUps: 'Suggested follow-ups',
  join: 'Join',
  joinTitle: 'Join the call and start recording',
  joinFailed: 'Could not join the call',
  records: 'Records automatically',
  skips: 'Will not record automatically',
  reasons,
  toggleFailed: 'Could not change auto-record',
  alwaysRecord: 'Record',
  neverRecord: 'Don’t record',
  justThisOne: 'Just this one',
  allOccurrences: 'All occurrences',
  followRules: 'Use my settings',
  more: 'More actions',
  openNote: 'Open note',
  emailInvitees: 'Email invitees',
  openNoteFailed: 'Could not open the note',
} as const;
