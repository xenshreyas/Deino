/**
 * Mock of `@/lib/ipc/generated/search`: keyword search over the fixture
 * meetings and people (`mock-search.ts`), `@` suggestions, and a Search
 * index page whose model downloads in a few progress events.
 */
import type * as Generated from '@/lib/ipc/generated/search';

import { mockDatabase, mockPeople } from './database';
import { latency } from './latency';
import { ok } from './mock-database';
import { mockEvent } from './mock-event';
import { fixtureDocuments, matchesAll, type MockDocument, queryWords, toHit } from './mock-search';

/** Progress events a mock download sends before it completes. */
const PROGRESS_STEPS = 4;
/** The most `@` suggestions, like `deino_core::search::MAX_SUGGESTIONS`. */
const MAX_SUGGESTIONS = 8;

export const events = {
  indexProgress: mockEvent<Generated.IndexProgress>('plugin:search:index-progress'),
  embeddingModelProgress: mockEvent<Generated.EmbeddingModelProgress>(
    'plugin:search:embedding-model-progress',
  ),
  embeddingModelChanged: mockEvent<Generated.EmbeddingModelChanged>(
    'plugin:search:embedding-model-changed',
  ),
} satisfies typeof Generated.events;

const model: Generated.ModelFileView = {
  id: 'nomic-embed-text-v1.5.Q8_0' as Generated.ModelFileId,
  name: 'Nomic Embed Text v1.5',
  quant: 'Q8_0',
  sizeBytes: 146_146_432,
  state: { kind: 'missing' },
  recommended: true,
};
let indexed = 0;

/** Every searchable document: meeting titles, transcripts and people. */
function documents(): MockDocument[] {
  const meetings = mockDatabase.meetings().flatMap((meeting) => {
    const fixture = mockDatabase.fixture(meeting.id);
    return fixture === undefined
      ? []
      : fixtureDocuments({ ...fixture, title: meeting.title }, meeting.id);
  });
  const people = mockPeople.map((person): MockDocument => ({
    kind: 'person',
    refId: person.id,
    meetingId: null,
    tsMs: null,
    title: person.name,
    body: person.email ?? '',
    date: null,
  }));
  return [...meetings, ...people];
}

/** The meetings a person filter allows: those the person attended. */
function allowedMeetings(filters: Generated.SearchFilters): Set<string> | null {
  const people = filters.chips.filter((chip) => chip.kind === 'person');
  if (people.length === 0) return null;
  const emails = people.map((chip) => mockPeople.find((person) => person.id === chip.id)?.email);
  const allowed = mockDatabase.meetings().filter((meeting) => {
    const attendees = mockDatabase.fixture(meeting.id)?.attendees ?? [];
    return emails.every((email) => attendees.some((attendee) => attendee.email === email));
  });
  return new Set(allowed.map((meeting) => meeting.id));
}

function search(request: Generated.SearchRequest): Generated.SearchResults {
  const words = queryWords(request.text);
  const filters = request.filters ?? { chips: [], domains: [] };
  const allowed = allowedMeetings(filters);
  const titles = new Map(mockDatabase.meetings().map((meeting) => [meeting.id, meeting.title]));
  const hits = documents()
    .filter(
      (document) =>
        allowed === null || (document.meetingId !== null && allowed.has(document.meetingId)),
    )
    .flatMap((document) => {
      if (words.length === 0) return document.kind === 'meeting' ? [toHit(document, [], '')] : [];
      const title = document.meetingId === null ? '' : (titles.get(document.meetingId) ?? '');
      return [toHit(document, words, title)];
    })
    .filter(
      (hit): hit is Generated.Hit => hit !== null && (words.length > 0 || hit.kind === 'meeting'),
    )
    .sort((a, b) => b.score - a.score);
  return sections(hits, request.limit, filters);
}

/** Groups hits like `deino_core::search::sections::group`. */
function sections(
  hits: Generated.Hit[],
  limit: number,
  filters: Generated.SearchFilters,
): Generated.SearchResults {
  const seen = new Set<string>();
  const meetings = hits.filter((hit) => {
    if (hit.meetingId === null || seen.has(hit.meetingId)) return false;
    seen.add(hit.meetingId);
    return true;
  });
  const take = (kind: Generated.HitKind) => hits.filter((hit) => hit.kind === kind).slice(0, limit);
  return {
    meetings: meetings.slice(0, limit),
    moments: take('segment'),
    people: take('person'),
    companies: [],
    actionItems: [],
    folders: [],
    filters,
    semantic: false,
  };
}

function suggestions(prefix: string): Generated.FilterChip[] {
  const people = mockPeople.map((person) => ({
    kind: 'person' as const,
    id: person.id,
    label: person.name,
  }));
  const meetings = mockDatabase
    .meetings()
    .map((meeting) => ({ kind: 'meeting' as const, id: meeting.id, label: meeting.title }));
  const words = queryWords(prefix);
  return [...people, ...meetings]
    .filter((chip) => words.length === 0 || matchesAll(chip.label, words))
    .slice(0, MAX_SUGGESTIONS);
}

function status(): Generated.IndexStatus {
  const total = mockDatabase.meetings().length;
  return {
    model: { ...model },
    totalMeetings: total,
    indexedMeetings: model.state.kind === 'ready' ? indexed : 0,
    backfill: { running: false, done: indexed, total, failed: 0 },
  };
}

async function download(): Promise<void> {
  for (let step = 1; step <= PROGRESS_STEPS; step += 1) {
    const downloadedBytes = Math.round((model.sizeBytes * step) / PROGRESS_STEPS);
    model.state = { kind: 'downloading', downloadedBytes };
    await events.embeddingModelProgress.emit({
      progress: { id: model.id, downloadedBytes, totalBytes: model.sizeBytes },
    });
    await latency();
  }
  model.state = { kind: 'ready' };
  indexed = mockDatabase.meetings().length;
  await events.embeddingModelChanged.emit({ id: model.id });
}

export const commands = {
  async search(request) {
    await latency();
    return ok(search(request));
  },
  async suggestFilters(prefix) {
    await latency();
    return ok(suggestions(prefix));
  },
  async reindexAll() {
    await latency();
    return ok({ documents: documents().length });
  },
  async indexStatus() {
    await latency();
    return ok(status());
  },
  async downloadModel() {
    await download();
    return ok(status());
  },
  async cancelDownload() {
    await latency();
    return ok(model.state.kind === 'downloading');
  },
  async deleteModel() {
    await latency();
    model.state = { kind: 'missing' };
    await events.embeddingModelChanged.emit({ id: model.id });
    return ok(status());
  },
  async startBackfill() {
    await latency();
    return ok(status());
  },
} satisfies typeof Generated.commands;
