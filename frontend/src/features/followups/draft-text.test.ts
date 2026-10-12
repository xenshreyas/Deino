import { describe, expect, it } from 'vitest';

import type { MeetingId } from '@/lib/ipc/generated/meetings';

import {
  editOf,
  fieldsOf,
  isDirty,
  placeholderCount,
  splitAddresses,
  textRuns,
} from './draft-text';
import type { FollowUpDraft } from './types';

const draft: FollowUpDraft = {
  meetingId: 'meeting-1' as MeetingId,
  subject: 'Pilot next steps',
  body: 'Hi [first name],',
  recipients: { to: ['marcus@brightkite.example', 'hannah@brightkite.example'], cc: [], bcc: [] },
  generatedAt: '2026-09-30T18:00:00Z',
  editedAt: null,
};

describe('textRuns', () => {
  it('splits placeholders out of the text the way Rust finds them', () => {
    const runs = textRuns('Hi [first name], see [the deck](https://x.y) by [date].\n[unclosed');

    expect(runs.filter((run) => run.placeholder).map((run) => run.text)).toEqual([
      '[first name]',
      '[date]',
    ]);
    expect(runs.map((run) => run.text).join('')).toBe(
      'Hi [first name], see [the deck](https://x.y) by [date].\n[unclosed',
    );
  });

  it('counts the gaps left and ignores empty or overlong brackets', () => {
    expect(placeholderCount('[a] and [b]')).toBe(2);
    expect(placeholderCount(`[] [${'x'.repeat(61)}]`)).toBe(0);
  });
});

describe('the editor fields', () => {
  it('join addresses for editing and split them back on commas and spaces', () => {
    const fields = fieldsOf(draft);

    expect(fields.to).toBe('marcus@brightkite.example, hannah@brightkite.example');
    expect(editOf({ ...fields, cc: ' lena@deino.dev;  priya@deino.dev ' }).cc).toEqual([
      'lena@deino.dev',
      'priya@deino.dev',
    ]);
    expect(splitAddresses(' , ')).toEqual([]);
  });

  it('are dirty once any field changes', () => {
    expect(isDirty(fieldsOf(draft), draft)).toBe(false);
    expect(isDirty({ ...fieldsOf(draft), body: 'Hi Marcus,' }, draft)).toBe(true);
  });
});
