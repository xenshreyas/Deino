/**
 * Orders the people Rust found for an @mention query. The search returns
 * matches by name; the menu puts the closest first: a name that starts with
 * the query, then a later word that does, then an email.
 */
import type { MentionCandidate, Person } from '../types';

/** The most people the menu shows. */
export const MENTION_LIMIT = 6;

/** 0 for a name prefix, 1 for a word prefix, 2 for an email prefix, 3 otherwise. */
function closeness(person: Person, query: string): number {
  const name = person.name.toLowerCase();
  if (name.startsWith(query)) return 0;
  if (name.split(/\s+/).some((word) => word.startsWith(query))) return 1;
  if (person.email?.toLowerCase().startsWith(query) === true) return 2;
  return 3;
}

/** The menu's candidates for `query`, closest first, at most {@link MENTION_LIMIT}. */
export function rankMentionCandidates(
  people: readonly Person[],
  query: string,
): MentionCandidate[] {
  const needle = query.trim().toLowerCase();
  return people
    .filter((person) => person.mergedInto === null)
    .map((person, index) => ({ person, index, rank: closeness(person, needle) }))
    .sort((a, b) => a.rank - b.rank || a.index - b.index)
    .slice(0, MENTION_LIMIT)
    .map(({ person }) => ({ id: person.id, label: person.name, detail: person.email }));
}

/** The shortest name worth offering to add, so `@a` does not invite "Add “a”". */
const MIN_NEW_NAME = 2;

/**
 * `candidates`, then an "Add “Name”" entry when the typed name matches
 * nobody exactly (ORG-04: people come from @mentions too). Choosing it adds
 * the person, or reuses a namesake Rust finds.
 */
export function withNewPerson(
  candidates: readonly MentionCandidate[],
  query: string,
): MentionCandidate[] {
  const name = query.trim().replace(/\s+/g, ' ');
  const known = candidates.some(
    (candidate) => candidate.label.toLowerCase() === name.toLowerCase(),
  );
  if (name.length < MIN_NEW_NAME || known) return [...candidates];
  return [...candidates, { id: `new:${name}`, label: name, detail: null, isNew: true }];
}
