/**
 * Mock of `@/lib/ipc/generated/vocabulary`: an in-memory vocabulary with the
 * Rust rules (trimmed, unique per kind ignoring case, a replacement needs
 * something to replace), and two corrections on the one-on-one fixture so
 * its Details panel has something to undo.
 */
import type * as Generated from '@/lib/ipc/generated/vocabulary';
import type { Table } from '@/lib/ipc/generated/shared';

import { failure, ok } from './command-result';
import { events } from './db';
import { latency } from './latency';

type Term = Generated.VocabTerm;

const ONE_ON_ONE = 'meeting-fixture-one-on-one' as Generated.MeetingId;
const SEEDED_AT = '2026-10-08T09:00:00.000Z';

let seq = 3_000_000;
function changed(tables: Table[]) {
  seq += 1;
  void events.dbChanged.emit({ tables, seq });
}

function term(id: string, kind: Generated.VocabKind, text: string, soundsLike: string[]): Term {
  const termId = id as Generated.VocabTermId;
  return { id: termId, kind, text, soundsLike, createdAt: SEEDED_AT, updatedAt: SEEDED_AT };
}

let terms: Term[] = [
  term('0192a5f0-0000-7000-8000-00000000a001', 'term', 'Shreyas', ['Sreyas', 'Shrey as']),
  term('0192a5f0-0000-7000-8000-00000000a002', 'term', 'PagerDuty', ['pager duty']),
  term('0192a5f0-0000-7000-8000-00000000a003', 'replacement', 'going to', ['gonna']),
];

let corrections: Generated.CorrectedSegment[] = [
  {
    segmentId: '0192a5f0-0000-7000-8000-00000000b001' as Generated.SegmentId,
    originalText: 'Thanks Sreyas, that plan works for me.',
    correctedText: 'Thanks Shreyas, that plan works for me.',
    undoable: true,
  },
  {
    segmentId: '0192a5f0-0000-7000-8000-00000000b002' as Generated.SegmentId,
    originalText: 'We are gonna page pager duty first.',
    correctedText: 'We are going to page PagerDuty first.',
    undoable: true,
  },
];

/** Rust's order: replacements, then terms, each by spelling ignoring case. */
function sorted(list: Term[]): Term[] {
  return [...list].sort(
    (a, b) =>
      a.kind.localeCompare(b.kind) || a.text.localeCompare(b.text, 'en', { sensitivity: 'base' }),
  );
}

/** The input as Rust validates it, or the error it answers with. */
function validated(input: Generated.TermInput, except: Generated.VocabTermId | null) {
  const text = input.text.trim().replace(/\s+/g, ' ');
  const soundsLike = input.soundsLike.map((v) => v.trim()).filter((v) => v.length > 0);
  if (text.length === 0)
    return failure<Term>({ code: 'invalid', message: 'a vocabulary entry cannot be empty' });
  if (input.kind === 'replacement' && soundsLike.length === 0)
    return failure<Term>({
      code: 'invalid',
      message: 'a replacement needs at least one word to replace',
    });
  const taken = terms.some(
    (t) => t.id !== except && t.kind === input.kind && t.text.toLowerCase() === text.toLowerCase(),
  );
  if (taken)
    return failure<Term>({ code: 'conflict', message: 'this word is already in your vocabulary' });
  return ok({ kind: input.kind, text, soundsLike });
}

export const commands = {
  async list() {
    await latency();
    return ok(sorted(terms));
  },
  async create(input) {
    await latency();
    const valid = validated(input, null);
    if (valid.status === 'error') return valid;
    const now = new Date().toISOString();
    const id = crypto.randomUUID() as Generated.VocabTermId;
    const created: Term = { ...valid.data, id, createdAt: now, updatedAt: now };
    terms = [...terms, created];
    changed(['vocab_terms', 'vocab_variants']);
    return ok(created);
  },
  async update(id, input) {
    await latency();
    const current = terms.find((t) => t.id === id);
    if (current === undefined)
      return failure<Term>({ code: 'not_found', message: `vocabulary entry ${id} does not exist` });
    const valid = validated(input, id);
    if (valid.status === 'error') return valid;
    const updated: Term = { ...current, ...valid.data, updatedAt: new Date().toISOString() };
    terms = terms.map((t) => (t.id === id ? updated : t));
    changed(['vocab_terms', 'vocab_variants']);
    return ok(updated);
  },
  async delete(id) {
    await latency();
    if (!terms.some((t) => t.id === id))
      return failure<null>({ code: 'not_found', message: `vocabulary entry ${id} does not exist` });
    terms = terms.filter((t) => t.id !== id);
    changed(['vocab_terms', 'vocab_variants']);
    return ok(null);
  },
  async corrections(meetingId) {
    await latency();
    return ok(meetingId === ONE_ON_ONE ? corrections : []);
  },
  async undo(meetingId, segmentId) {
    await latency();
    const undone = corrections.filter(
      (c) => meetingId === ONE_ON_ONE && (segmentId === null || c.segmentId === segmentId),
    );
    corrections = corrections.filter((c) => !undone.includes(c));
    changed(['vocab_corrections', 'transcript_segments']);
    const restored = undone.filter((c) => c.undoable).length;
    return ok({ restored, kept: undone.length - restored });
  },
} satisfies typeof Generated.commands;
