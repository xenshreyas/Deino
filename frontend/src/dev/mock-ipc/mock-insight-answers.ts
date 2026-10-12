/**
 * What the mock "model" answers when an insight runs on a fixture meeting:
 * deterministic, typed answers attributed to real transcript lines, so the
 * Insights tab and table look as they would with a real model.
 *
 * Free-text insights answer with the meeting's ground-truth decisions;
 * typed ones fill each field from the transcript by its type.
 */
import type * as Generated from '@/lib/ipc/generated/insights';

import { fixtureUuid } from './fixture-ids';
import type { FixtureMeeting } from './fixture-schema';

type Answer = Generated.InsightAnswer;
type Def = Generated.InsightDef;

/** How many answers an Auto insight gives in the mock. */
const AUTO_ANSWERS = 3;

/** The speaker of segment `index`, by name. */
function speakerOf(fixture: FixtureMeeting, index: number): string | null {
  const segment = fixture.segments[index];
  if (segment === undefined) return null;
  const speaker = fixture.speakers.find((candidate) => candidate.label === segment.speaker);
  const person = fixture.attendees.find((attendee) => attendee.email === speaker?.attendee);
  return person?.name ?? segment.speaker;
}

/** An answer of `value` said on segment `index`. */
function attributed(fixture: FixtureMeeting, index: number, value: Generated.InsightValue): Answer {
  const segment = fixture.segments[index];
  return {
    value,
    speaker: speakerOf(fixture, index),
    timestampMs: segment?.startMs ?? null,
    segmentId:
      segment === undefined
        ? null
        : (fixtureUuid(`segment/${fixture.id}/${String(index)}`) as Generated.SegmentId),
  };
}

/** A segment a little further into the meeting for each answer. */
function lineFor(fixture: FixtureMeeting, answer: number): number {
  const step = Math.max(1, Math.floor(fixture.segments.length / (AUTO_ANSWERS + 1)));
  return Math.min(fixture.segments.length - 1, step * (answer + 1));
}

/** A value of `field`'s type, from the transcript line `line`. */
function valueOf(
  field: Generated.InsightField,
  fixture: FixtureMeeting,
  line: number,
): Generated.FieldValue {
  const said = fixture.segments[line]?.text ?? fixture.title;
  switch (field.kind.type) {
    case 'number':
      return {
        type: 'number',
        value: /score/i.test(field.name) ? 3 + (line % 3) : 40_000 + line * 500,
      };
    case 'boolean':
      return { type: 'boolean', value: line % 2 === 0 };
    case 'date':
      return { type: 'date', value: fixture.startedAt.slice(0, 10) };
    case 'enum':
      return { type: 'choice', value: field.kind.options[line % field.kind.options.length] ?? '' };
    case 'text':
      return { type: 'text', value: said.length > 90 ? `${said.slice(0, 88).trimEnd()}…` : said };
  }
}

/** How many answers `def` keeps. */
function limitOf(def: Def): number {
  return def.cardinality.kind === 'fixed' ? def.cardinality.count : AUTO_ANSWERS;
}

/** The mock model's answers to `def` on `fixture`. */
export function mockAnswers(def: Def, fixture: FixtureMeeting): Answer[] {
  const count = Math.min(limitOf(def), AUTO_ANSWERS);
  if (def.fields.length === 0) {
    return fixture.groundTruth.decisions
      .slice(0, count)
      .map((decision, index) =>
        attributed(fixture, lineFor(fixture, index), { kind: 'text', text: decision }),
      );
  }
  return Array.from({ length: count }, (_, index) => {
    const line = lineFor(fixture, index);
    const fields = def.fields.map((field, at) => ({
      name: field.name,
      value: valueOf(field, fixture, Math.min(line + at, fixture.segments.length - 1)),
    }));
    return attributed(fixture, line, { kind: 'fields', fields });
  });
}

/** True when `conditions` match `fixture` (the mock knows titles, domains and audience). */
export function mockMatches(
  conditions: Generated.ConditionGroups,
  fixture: FixtureMeeting,
): boolean {
  if (conditions.length === 0) return true;
  const domains = fixture.attendees.map((attendee) => attendee.email.split('@')[1] ?? '');
  const external = domains.some((domain) => domain !== 'deino.dev');
  return conditions.some((group) =>
    group.every((condition) => {
      switch (condition.kind) {
        case 'external':
          return condition.external === external;
        case 'title_contains':
          return fixture.title.toLowerCase().includes(condition.text.toLowerCase());
        case 'attendee_domain':
          return domains.includes(condition.domain.toLowerCase());
        default:
          return false;
      }
    }),
  );
}
