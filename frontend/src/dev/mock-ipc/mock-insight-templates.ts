/**
 * The insight template gallery for `pnpm dev:mock`: the same twelve
 * templates as `deino_core::insights::gallery`, abridged descriptions.
 */
import type * as Generated from '@/lib/ipc/generated/insights';

type Field = Generated.InsightField;
type Template = Generated.InsightTemplate;

const text = (name: string): Field => ({ name, kind: { type: 'text' } });
const number = (name: string): Field => ({ name, kind: { type: 'number' } });
const yesNo = (name: string): Field => ({ name, kind: { type: 'boolean' } });
const date = (name: string): Field => ({ name, kind: { type: 'date' } });
const choice = (name: string, options: string[]): Field => ({
  name,
  kind: { type: 'enum', options },
});

const ONE: Generated.Cardinality = { kind: 'fixed', count: 1 };
const AUTO: Generated.Cardinality = { kind: 'auto' };
const ALWAYS: Generated.ApplyWhen = { mode: 'automatic', conditions: [] };
const EXTERNAL: Generated.ApplyWhen = {
  mode: 'automatic',
  conditions: [[{ kind: 'external', external: true }]],
};
const MANUAL: Generated.ApplyWhen = { mode: 'manual' };

function template(
  key: string,
  name: string,
  summary: string,
  rest: Pick<Generated.InsightInput, 'fields' | 'cardinality' | 'applyWhen'>,
): Template {
  return { key, summary, input: { name, description: summary, ...rest } };
}

/** The gallery, in Rust's order. */
export const MOCK_INSIGHT_TEMPLATES: Template[] = [
  template(
    'discovery_scorecard',
    'Discovery scorecard',
    'Qualify a prospect: pain, budget, timeline, decision-maker and a 1–5 score.',
    {
      fields: [
        text('Questions'),
        text('Pain'),
        number('Budget'),
        text('Timeline'),
        text('Decision-maker'),
        number('Score'),
      ],
      cardinality: ONE,
      applyWhen: EXTERNAL,
    },
  ),
  template(
    'customer_feedback',
    'Customer feedback',
    'Feature requests, complaints and praise, each with a quote.',
    {
      fields: [
        choice('Kind', ['Feature request', 'Complaint', 'Praise']),
        text('Feedback'),
        text('Quote'),
      ],
      cardinality: AUTO,
      applyWhen: EXTERNAL,
    },
  ),
  template(
    'competitor_mentions',
    'Competitor mentions',
    'Every competitor named, in what context and how it came across.',
    {
      fields: [
        text('Competitor'),
        text('Context'),
        choice('Sentiment', ['Positive', 'Neutral', 'Negative']),
      ],
      cardinality: AUTO,
      applyWhen: ALWAYS,
    },
  ),
  template(
    'health_signals',
    'Health signals',
    'Red, yellow or green account health, with the signals behind it.',
    {
      fields: [choice('Health', ['Red', 'Yellow', 'Green']), text('Signals'), yesNo('Churn risk')],
      cardinality: ONE,
      applyWhen: EXTERNAL,
    },
  ),
  template(
    'executive_summary',
    'Executive summary',
    'Three to five sentences an executive can read in thirty seconds.',
    {
      fields: [],
      cardinality: ONE,
      applyWhen: MANUAL,
    },
  ),
  template(
    'interview_debrief',
    'Interview debrief',
    'Strengths, concerns and a hiring recommendation for a candidate.',
    {
      fields: [
        text('Candidate'),
        text('Role'),
        text('Strengths'),
        text('Concerns'),
        choice('Recommendation', ['Strong yes', 'Yes', 'No', 'Strong no']),
        number('Score'),
      ],
      cardinality: ONE,
      applyWhen: {
        mode: 'automatic',
        conditions: [[{ kind: 'title_contains', text: 'interview' }]],
      },
    },
  ),
  template('questions_asked', 'Questions asked', 'Every substantive question, one answer each.', {
    fields: [],
    cardinality: AUTO,
    applyWhen: MANUAL,
  }),
  template(
    'objections',
    'Objections',
    'Pushback raised, how it was answered, and whether it was settled.',
    {
      fields: [text('Objection'), text('Response'), yesNo('Resolved')],
      cardinality: AUTO,
      applyWhen: EXTERNAL,
    },
  ),
  template('risks', 'Risks', 'Risks to the work, with severity and an owner.', {
    fields: [text('Risk'), choice('Severity', ['Low', 'Medium', 'High']), text('Owner')],
    cardinality: AUTO,
    applyWhen: ALWAYS,
  }),
  template('decisions', 'Decisions', 'What was decided, one decision per answer.', {
    fields: [],
    cardinality: AUTO,
    applyWhen: ALWAYS,
  }),
  template(
    'follow_up_email',
    'Follow-up email',
    'A ready-to-send recap email to the other attendees.',
    {
      fields: [],
      cardinality: ONE,
      applyWhen: MANUAL,
    },
  ),
  template(
    'deal_summary',
    'Deal summary for CRM',
    'Company, stage, size, close date and next step, ready for a CRM.',
    {
      fields: [
        text('Company'),
        choice('Stage', [
          'Discovery',
          'Evaluation',
          'Proposal',
          'Negotiation',
          'Closed won',
          'Closed lost',
        ]),
        number('Deal size'),
        date('Close date'),
        text('Next step'),
        text('Summary'),
      ],
      cardinality: ONE,
      applyWhen: EXTERNAL,
    },
  ),
];
