import { describe, expect, it } from 'vitest';

import { fromDraft, move, newRule, patchItem, toDraft } from './draft';
import type { TemplateContent } from './types';

const content: TemplateContent = {
  name: 'Sales discovery',
  description: 'A first call',
  meetingContext: 'Selling',
  sections: [
    { title: 'Pain points', instructions: 'Problems', format: 'bullets' },
    { title: 'Next steps', instructions: '', format: 'todo' },
  ],
  autoRules: [
    {
      label: 'external discovery',
      titlePattern: 'discovery',
      attendeeDomains: ['acme.com', 'globex.com'],
      audience: 'external',
      attendeeCount: { min: 2, max: null },
    },
  ],
};

describe('template drafts', () => {
  it('round-trip saved content unchanged', () => {
    expect(fromDraft(toDraft(content))).toEqual(content);
  });

  it('give every section and rule its own stable key', () => {
    const draft = toDraft(content);

    const keys = [...draft.sections, ...draft.rules].map(({ key }) => key);

    expect(new Set(keys).size).toBe(3);
  });

  it('turn typed rule fields into a rule, blank meaning no condition', () => {
    const typed = { ...newRule(), domains: ' acme.com,  globex.com ', max: '3', min: 'x' };

    const [rule] = fromDraft({ ...toDraft(content), rules: [typed] }).autoRules;

    expect(rule).toEqual({
      label: '',
      titlePattern: null,
      attendeeDomains: ['acme.com', 'globex.com'],
      audience: null,
      attendeeCount: { min: null, max: 3 },
    });
  });

  it('move items within bounds and patch them by key', () => {
    expect(move(['a', 'b', 'c'], 0, 2)).toEqual(['b', 'c', 'a']);
    expect(move(['a', 'b', 'c'], 2, -5)).toEqual(['c', 'a', 'b']);
    expect(move(['a', 'b'], 1, 1)).toEqual(['a', 'b']);
    const items = [
      { key: 'x', title: 'One' },
      { key: 'y', title: 'Two' },
    ];
    expect(patchItem(items, 'y', { title: 'Deux' })[1]).toEqual({ key: 'y', title: 'Deux' });
  });
});
