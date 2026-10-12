/**
 * Mock of `@/lib/ipc/generated/briefs`: the sales discovery fixture's note
 * carries a brief (as if its event were still coming up), citing the
 * fixture library, so `pnpm dev:mock` shows the card. Reading and the
 * thumbs down change it in memory, like Rust. `?brief=none` hides it.
 *
 * It imports no feature code, so the feature reaching it through its
 * generated bindings forms no cycle.
 */
import type * as Generated from '@/lib/ipc/generated/briefs';

import { ok } from './command-result';
import { fixtureUuid } from './fixture-ids';
import { latency } from './latency';

/** The fixture meeting whose note shows the brief. */
const MEETING = 'meeting-fixture-sales-discovery';
/** Its calendar event, as the Rust seeder names it. */
const EVENT = fixtureUuid(`event/${MEETING}`) as Generated.CalendarEventId;

/** The brief, until the page changes it. */
let brief: Generated.Brief = {
  eventId: EVENT,
  bullets: [
    {
      text: "Brightkite's DispatchPro licence ends on 31 January 2027, so a decision is needed before then.",
      citations: [
        {
          kind: 'meeting',
          meetingId: MEETING as Generated.MeetingId,
          title: 'Deino × Brightkite Logistics: discovery',
          at: '2026-09-30T15:01:24Z',
        },
      ],
    },
    {
      text: 'Marcus Webb runs operations and wants dispatchers trained in under a week.',
      citations: [
        {
          kind: 'person',
          personId: fixtureUuid(
            'person/marcus.webb@brightkite-logistics.com',
          ) as Generated.PersonId,
          name: 'Marcus Webb',
        },
      ],
    },
    {
      text: 'You still owe Hannah the security questionnaire; the agenda lists pricing and next steps.',
      citations: [
        {
          kind: 'moment',
          meetingId: MEETING as Generated.MeetingId,
          segmentId: fixtureUuid(`segment/${MEETING}/40`) as Generated.SegmentId,
          label: 'Send the security questionnaire',
        },
        { kind: 'event' },
      ],
    },
  ],
  generatedAt: '2026-10-12T13:00:00Z',
  readAt: null,
  feedback: null,
};

/** Whether the page asked for no brief (`?brief=none`). */
function hidden(): boolean {
  return new URLSearchParams(window.location.search).get('brief') === 'none';
}

export const commands = {
  async forEvent(event) {
    await latency();
    return ok(event === EVENT && !hidden() ? brief : null);
  },
  async forMeeting(meeting) {
    await latency();
    return ok(meeting === MEETING && !hidden() ? brief : null);
  },
  async markRead(event) {
    await latency();
    if (event === EVENT && brief.readAt === null) {
      brief = { ...brief, readAt: new Date().toISOString() };
    }
    return ok(null);
  },
  async giveFeedback(event, feedback) {
    await latency();
    if (event === EVENT) brief = { ...brief, feedback };
    return ok(null);
  },
} satisfies typeof Generated.commands;
