/**
 * A few of Deino's built-in templates for `pnpm dev:mock`, shaped like the
 * Rust catalogue (`deino_core::templates::CATALOG`, whose text is the source
 * of truth): enough to browse, duplicate and pick in the UI.
 */
import type { AutoRule, SectionFormat, Template, TemplateId } from '@/lib/ipc/generated/templates';

const SEEDED_AT = '2026-10-01T09:00:00.000Z';

type Section = [title: string, format: SectionFormat, instructions: string];

function rule(label: string, titlePattern: string): AutoRule {
  return { label, titlePattern, attendeeDomains: [], audience: null, attendeeCount: null };
}

function builtin(
  id: string,
  [name, description, meetingContext]: [string, string, string],
  sections: Section[],
  autoRules: AutoRule[] = [],
): Template {
  return {
    id: id as TemplateId,
    isBuiltin: true,
    content: {
      name,
      description,
      meetingContext,
      sections: sections.map(([title, format, instructions]) => ({ title, format, instructions })),
      autoRules,
    },
    createdAt: SEEDED_AT,
    updatedAt: SEEDED_AT,
  };
}

/** The mock's built-ins, by name. */
export const BUILT_IN_TEMPLATES: readonly Template[] = [
  builtin(
    'one_on_one',
    [
      '1:1',
      'A recurring one-on-one between a manager and a report, or two peers.',
      'Two people catching up regularly.',
    ],
    [
      ['Check-in', 'paragraph', 'How each person is doing, as far as they shared it.'],
      ['Topics', 'bullets', 'Each topic either person raised, and where it landed.'],
      ['Action items', 'todo', 'Follow-ups for either person, with the owner and due date.'],
    ],
    [rule('1:1 in the title', String.raw`\b1\s?[:\-/]\s?1\b`)],
  ),
  builtin(
    'circleback_style',
    [
      'Circleback style',
      'Action items first, a three-to-six bullet overview, then one section per topic discussed.',
      'Notes for someone who missed the meeting.',
    ],
    [
      [
        'Action Items',
        'todo',
        "Every task someone took on: the owner's name, the task and the due date.",
      ],
      ['Overview', 'bullets', 'Three to six bullets summarising the whole meeting.'],
      ['Topics', 'bullets', 'One subsection per discussion topic, in the order discussed.'],
    ],
  ),
  builtin(
    'general',
    [
      'General',
      'Adapts to any meeting: an overview, the discussion by topic, decisions and next steps.',
      'Use when no more specific template fits.',
    ],
    [
      [
        'Overview',
        'bullets',
        'Three to five bullets: why the meeting happened and what was concluded.',
      ],
      ['Discussion', 'bullets', 'The conversation grouped by topic.'],
      ['Decisions', 'bullets', 'Each decision with its reason.'],
      ['Next steps', 'todo', 'Every commitment, with the owner and due date.'],
    ],
  ),
  builtin(
    'sales_discovery',
    [
      'Sales discovery',
      'A first call with a prospect: their situation, pain, buying process and fit.',
      "The user's team is selling to a prospect.",
    ],
    [
      ['Pain points', 'bullets', 'Problems they described, with their impact.'],
      ['Buying process', 'table', 'Decision makers, budget, timeline, competitors.'],
      ['Next steps', 'todo', 'Next steps agreed on both sides.'],
    ],
    [rule('external discovery call', String.raw`\b(discovery|intro)\b`)],
  ),
  builtin(
    'standup',
    [
      'Stand-up',
      'A short daily stand-up: progress, plans and blockers per person.',
      'A time-boxed team check-in.',
    ],
    [
      ['Updates', 'table', 'One row per person: done, next.'],
      ['Blockers', 'bullets', 'Each blocker and who is blocked.'],
    ],
    [rule('stand-up', String.raw`\bstand[\s-]?up\b`)],
  ),
];
