/**
 * The Stats page's numbers in dev:mock: the eight fixture meetings plus a
 * deterministic year of weekday meetings with the same people, so the
 * heatmap and charts have a realistic shape. The rules mirror
 * `deino_core::stats` (declined invitees and the user left out, internal by
 * the user's domain); the LLM calls are derived from the meetings.
 */
import type * as Generated from '@/lib/ipc/generated/stats';

import { fixtureUuid } from './fixture-ids';
import type { FixtureMeeting } from './fixture-schema';

const USER = 'shreyas@deino.dev';
const INTERNAL = '@deino.dev';
const DAY_MS = 86_400_000;
const PRICES: Record<string, [number, number]> = {
  'claude-sonnet-5-5': [2, 10],
  'claude-haiku-5-5': [0.1, 0.5],
};

/** One counted meeting. */
interface Fact {
  at: Date;
  seconds: number;
  people: { email: string; name: string }[];
}

function fromFixture(fixture: FixtureMeeting): Fact {
  return {
    at: new Date(fixture.startedAt),
    seconds: Math.round(fixture.durationMs / 1000),
    people: fixture.attendees.filter((attendee) => attendee.rsvp !== 'declined'),
  };
}

/** A small seeded generator, so every reload draws the same year. */
function random(seed: number): () => number {
  let state = seed;
  return () => {
    state = (state * 1_103_515_245 + 12_345) % 2_147_483_648;
    return state / 2_147_483_648;
  };
}

/** Weekday meetings over the year before `now`, with the fixtures' people. */
function history(fixtures: readonly FixtureMeeting[], now: Date): Fact[] {
  const pool = fixtures.flatMap((fixture) => fixture.attendees).filter((a) => a.email !== USER);
  const next = random(9);
  const facts: Fact[] = [];
  for (let back = 21; back < 365; back += 1) {
    const day = new Date(now.getTime() - back * DAY_MS);
    const weekday = day.getUTCDay();
    const count = weekday === 0 || weekday === 6 ? 0 : Math.floor(next() * (weekday === 5 ? 2 : 4));
    for (let index = 0; index < count; index += 1) {
      const at = new Date(day);
      at.setUTCHours(9 + Math.floor(next() * 8), 0, 0, 0);
      const someone = pool[Math.floor(next() * pool.length)];
      const people = someone === undefined ? [] : [someone];
      facts.push({ at, seconds: 900 + Math.floor(next() * 2700), people });
    }
  }
  return facts;
}

/** Every counted meeting in dev:mock. */
export function mockFacts(fixtures: readonly FixtureMeeting[], now: Date): Fact[] {
  return [...fixtures.map(fromFixture), ...history(fixtures, now)];
}

/** Where `range` starts, at UTC midnight. */
export function sinceOf(range: Generated.StatsRange, now: Date): Date | null {
  const days = { last_30_days: 29, last_90_days: 89, last_12_months: 364, all_time: null }[range];
  if (days === null) return null;
  const today = Date.UTC(now.getUTCFullYear(), now.getUTCMonth(), now.getUTCDate());
  return new Date(today - days * DAY_MS);
}

function people(facts: readonly Fact[]): [number, Generated.TopPerson[]] {
  const met = new Map<string, Generated.TopPerson>();
  for (const person of facts.flatMap((fact) => fact.people)) {
    if (person.email === USER) continue;
    const id = fixtureUuid(`person/${person.email}`) as Generated.PersonId;
    const entry = met.get(person.email) ?? { id, name: person.name, meetings: 0 };
    met.set(person.email, { ...entry, meetings: entry.meetings + 1 });
  }
  const ranked = [...met.values()].sort(
    (a, b) => b.meetings - a.meetings || a.name.localeCompare(b.name),
  );
  return [met.size, ranked.slice(0, 8)];
}

function audience(facts: readonly Fact[]): Generated.AudienceSplit {
  const split = { internal: 0, external: 0, unknown: 0 };
  for (const fact of facts) {
    if (fact.people.length === 0) split.unknown += 1;
    else if (fact.people.every((person) => person.email.endsWith(INTERNAL))) split.internal += 1;
    else split.external += 1;
  }
  return split;
}

function usageRow(model: string, calls: number, [input, output]: [number, number]) {
  const price = PRICES[model];
  const provider: Generated.ProviderId = price === undefined ? 'builtin' : 'anthropic';
  const cost = price === undefined ? 0 : Math.round(input * price[0] + output * price[1]);
  return {
    provider,
    model,
    calls,
    failed: Math.floor(calls / 40),
    inputTokens: input,
    outputTokens: output,
    estimatedCostMicros: cost,
  };
}

function llm(meetings: number): Generated.LlmUsage {
  const models = [
    usageRow('claude-haiku-5-5', meetings * 3, [meetings * 3 * 2_400, meetings * 3 * 120]),
    usageRow('claude-sonnet-5-5', meetings, [meetings * 14_000, meetings * 1_100]),
    usageRow('nomic-embed-text-v1.5', meetings * 2, [meetings * 9_000, 0]),
  ].filter((row) => row.calls > 0);
  const total = models.reduce((sum, row) => sum + row.estimatedCostMicros, 0);
  return { models, estimatedCostMicros: total, unpricedCalls: 0 };
}

function tally(facts: readonly Fact[], slots: number, slot: (at: Date) => number): number[] {
  const counts = Array.from({ length: slots }, () => 0);
  for (const fact of facts) counts[slot(fact.at)] = (counts[slot(fact.at)] ?? 0) + 1;
  return counts;
}

/** The numbers of `facts` since `since`. */
export function overviewOf(facts: readonly Fact[], since: Date | null): Generated.StatsOverview {
  const shown = facts.filter((fact) => since === null || fact.at >= since);
  const [met, topPeople] = people(shown);
  return {
    since: since?.toISOString().replace('.000Z', 'Z') ?? null,
    meetings: shown.length,
    recordedSeconds: shown.reduce((sum, fact) => sum + fact.seconds, 0),
    people: met,
    audience: audience(shown),
    topPeople,
    weekdays: tally(shown, 7, (at) => (at.getUTCDay() + 6) % 7),
    hours: tally(shown, 24, (at) => at.getUTCHours()),
    llm: llm(shown.length),
  };
}

/** Meetings per day over the 53 weeks ending `now`. */
export function heatmapOf(facts: readonly Fact[], now: Date): Generated.Heatmap {
  const today = Date.UTC(now.getUTCFullYear(), now.getUTCMonth(), now.getUTCDate());
  const back = 364 + ((now.getUTCDay() + 6) % 7);
  const first = today - back * DAY_MS;
  const counts = new Map<string, number>();
  for (const fact of facts) {
    if (fact.at.getTime() < first) continue;
    const day = fact.at.toISOString().slice(0, 10);
    counts.set(day, (counts.get(day) ?? 0) + 1);
  }
  const days = [...counts.entries()]
    .sort(([a], [b]) => a.localeCompare(b))
    .map(([day, meetings]) => ({ day, meetings }));
  const iso = (time: number) => new Date(time).toISOString().slice(0, 10);
  return { firstDay: iso(first), lastDay: iso(today), days };
}
