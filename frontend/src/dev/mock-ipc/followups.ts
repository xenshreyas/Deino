/**
 * Mock of `@/lib/ipc/generated/followups`: the sales discovery fixture has
 * a ready follow-up draft to its two Brightkite guests, so `pnpm dev:mock`
 * shows Home's "Suggested follow-ups" and the meeting's banner. Saving,
 * Mail, Copy and dismissing change it in memory, like Rust (Mail opens
 * nothing). `?followup=none` hides it.
 *
 * It imports no feature code, so the feature reaching it through its
 * generated bindings forms no cycle.
 */
import type * as Generated from '@/lib/ipc/generated/followups';

import { ok } from './command-result';
import { latency } from './latency';

/** The fixture meeting with the draft. */
const MEETING = 'meeting-fixture-sales-discovery' as Generated.MeetingId;
/** Its title, as the fixture names it. */
const TITLE = 'Deino × Brightkite Logistics: discovery';

/** The draft, until the page changes or dismisses it. */
let draft: Generated.FollowUpDraft | null = {
  meetingId: MEETING,
  subject: 'Brightkite pilot: proposal and next steps',
  body: [
    'Hi Marcus and Hannah,',
    '',
    'Thanks for walking us through dispatch at Brightkite today. To recap what we agreed:',
    '',
    '- A paid 60-day pilot in one region with about 70 technicians, credited against the first year if you sign.',
    '- Hannah’s security review comes first: I’ll send our SOC 2 Type I report and pentest summary by [date].',
    '- I’ll share the pilot proposal on the enterprise tier by [proposal date].',
    '',
    'Would [day and time] work for a short call to walk through the proposal?',
    '',
    'Best,',
    'Shreyas',
  ].join('\n'),
  recipients: {
    to: ['marcus.webb@brightkite-logistics.com', 'hannah.pike@brightkite-logistics.com'],
    cc: [],
    bcc: [],
  },
  generatedAt: new Date(Date.now() - 2 * 60 * 60 * 1000).toISOString(),
  editedAt: null,
};

/** Whether the page asked for no follow-up (`?followup=none`). */
function hidden(): boolean {
  return new URLSearchParams(window.location.search).get('followup') === 'none';
}

/** The draft when it is shown for `meeting`. */
function shown(meeting: Generated.MeetingId): Generated.FollowUpDraft | null {
  return meeting === MEETING && !hidden() ? draft : null;
}

/** Applies an edit as Rust's `save` does (addresses unchecked here). */
function apply(edit: Generated.FollowUpEdit): Generated.FollowUpDraft | null {
  if (draft === null) return null;
  draft = {
    ...draft,
    subject: edit.subject,
    body: edit.body,
    recipients: { to: edit.to, cc: edit.cc, bcc: edit.bcc },
    editedAt: new Date().toISOString(),
  };
  return draft;
}

export const commands = {
  async suggested() {
    await latency();
    const ready = draft === null || hidden() ? [] : [draft];
    return ok(
      ready.map((item) => ({
        meetingId: item.meetingId,
        meetingTitle: TITLE,
        subject: item.subject,
        generatedAt: item.generatedAt,
      })),
    );
  },
  async forMeeting(meeting) {
    await latency();
    return ok(shown(meeting));
  },
  async save(_meeting, edit) {
    await latency();
    const saved = apply(edit);
    if (saved === null) {
      return { status: 'error', error: { code: 'not_found', message: 'No follow-up draft' } };
    }
    return ok(saved);
  },
  async dismiss() {
    await latency();
    draft = null;
    return ok(null);
  },
  async openInMail(_meeting, edit) {
    await latency();
    apply(edit);
    return ok(null);
  },
  async copy(_meeting, edit) {
    await latency();
    const saved = apply(edit);
    if (saved !== null) await navigator.clipboard.writeText(saved.body).catch(() => undefined);
    return ok(null);
  },
} satisfies typeof Generated.commands;
