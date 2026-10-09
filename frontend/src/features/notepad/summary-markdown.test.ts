import { describe, expect, it } from 'vitest';

import { summaryMarkdown } from './summary-markdown';

describe('summaryMarkdown', () => {
  it('passes Markdown summaries through', () => {
    expect(summaryMarkdown({ markdown: '# Plan\n\n- ship' })).toBe('# Plan\n\n- ship');
  });

  it('reads summaries the BlockNote editor saved', () => {
    const summaryJson = [
      { type: 'heading', props: { level: 2 }, content: [{ type: 'text', text: 'Decisions' }] },
      {
        type: 'bulletListItem',
        content: [
          { type: 'text', text: 'Ship ', styles: {} },
          { type: 'text', text: 'Friday', styles: { bold: true } },
        ],
        children: [{ type: 'bulletListItem', content: [{ type: 'text', text: 'after QA' }] }],
      },
      {
        type: 'checkListItem',
        props: { checked: true },
        content: [{ type: 'text', text: 'Book room' }],
      },
      {
        type: 'paragraph',
        content: [{ type: 'link', href: 'https://a.io', content: [{ type: 'text', text: 'doc' }] }],
      },
    ];

    expect(summaryMarkdown({ summary_json: summaryJson })).toBe(
      '## Decisions\n\n- Ship **Friday**\n  - after QA\n- [x] Book room\n\n[doc](https://a.io)',
    );
  });

  it('reads the first app section format in its saved order', () => {
    const legacy = {
      MeetingName: 'Standup',
      _section_order: ['ActionItems', 'Decisions'],
      Decisions: {
        title: 'Decisions',
        blocks: [{ id: '1', type: 'text', content: 'Ship it', color: '' }],
      },
      ActionItems: {
        title: 'Action Items',
        blocks: [{ id: '2', type: 'bullet', content: 'Send deck', color: '' }],
      },
    };

    expect(summaryMarkdown(legacy)).toBe(
      '## Action Items\n\n- Send deck\n\n## Decisions\n\nShip it',
    );
  });

  it('is empty for nothing it can read', () => {
    expect(summaryMarkdown(null)).toBe('');
    expect(summaryMarkdown('text')).toBe('');
  });
});
