import { describe, expect, it, vi } from 'vitest';

import type { Template, TemplateId } from '../types';
import {
  matchTemplates,
  type ScaffoldChain,
  scaffoldContent,
  templateQuery,
  templateSlashSuggestion,
} from './template-slash';

function template(id: string, name: string, sections: string[]): Template {
  return {
    id: id as TemplateId,
    isBuiltin: true,
    content: {
      name,
      description: `${name} notes`,
      meetingContext: '',
      sections: sections.map((title) => ({ title, instructions: '', format: 'bullets' })),
      autoRules: [],
    },
    createdAt: '2026-10-01T09:00:00Z',
    updatedAt: '2026-10-01T09:00:00Z',
  };
}

const templates = [
  template('general', 'General', ['Overview']),
  template('sales_discovery', 'Sales discovery', ['Pain points', 'Next steps']),
  template('sales_demo', 'Sales demo', ['What was shown']),
  template('one_on_one', '1:1', ['Topics']),
];

/** A chain that records what the command does to the document. */
function recordingEditor() {
  const steps: unknown[] = [];
  const record = (step: unknown) => {
    steps.push(step);
    return chain;
  };
  const chain: ScaffoldChain = {
    focus: () => record('focus'),
    deleteRange: record,
    insertContent: record,
    run: () => true,
  };
  return { editor: { chain: () => chain }, steps };
}

describe('the /template command', () => {
  it('opens on "template" and its prefixes, filtering by what follows', () => {
    expect(templateQuery('temp')).toBe('');
    expect(templateQuery('template Sales  disc')).toBe('sales disc');
    expect(templateQuery('heading')).toBeNull();
    expect(templateQuery('temp sales')).toBeNull();
  });

  it('lists names that start with the filter first', () => {
    const names = matchTemplates(templates, 'sales').map(({ content }) => content.name);

    expect(names).toEqual(['Sales discovery', 'Sales demo']);
    expect(matchTemplates(templates, 'notes')).toHaveLength(4);
  });

  it('sets the meeting template and replaces the command with section headings', () => {
    const setMeetingTemplate = vi.fn<(id: TemplateId) => void>();
    const suggestion = templateSlashSuggestion({ templates: () => templates, setMeetingTemplate });
    const { editor, steps } = recordingEditor();
    const [discovery] = suggestion.items({ query: 'template disc' });
    if (discovery === undefined) throw new Error('no match');

    suggestion.command({ editor, range: { from: 3, to: 17 }, props: discovery });

    expect(setMeetingTemplate).toHaveBeenCalledWith('sales_discovery');
    expect(steps).toEqual(['focus', { from: 3, to: 17 }, scaffoldContent(discovery)]);
    expect(scaffoldContent(discovery)).toEqual([
      { type: 'heading', attrs: { level: 2 }, content: [{ type: 'text', text: 'Pain points' }] },
      { type: 'paragraph' },
      { type: 'heading', attrs: { level: 2 }, content: [{ type: 'text', text: 'Next steps' }] },
      { type: 'paragraph' },
    ]);
  });
});
