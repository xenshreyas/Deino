/**
 * Mock of `@/lib/ipc/generated/importers` (CAP-12): Granola's key already
 * saved, the others missing. A preview counts a fixed library; an import
 * ticks through it on a timer and emits its progress like the real job.
 * "Stop" ends it after the current meeting.
 */
import type * as Generated from '@/lib/ipc/generated/importers';

import { failure, ok } from './command-result';
import { latency } from './latency';
import { mockEvent } from './mock-event';

type ImportSource = Generated.ImportSource;
type ImportProgress = Generated.ImportProgress;

const SOURCES: readonly ImportSource[] = ['granola', 'circleback', 'fireflies', 'fathom', 'folder'];
/** What each source "has": total meetings and those already imported. */
const LIBRARY: Record<ImportSource, { total: number; already: number }> = {
  granola: { total: 48, already: 12 },
  circleback: { total: 20, already: 0 },
  fireflies: { total: 9, already: 0 },
  fathom: { total: 31, already: 31 },
  folder: { total: 6, already: 0 },
};
const TICK_MS = 250;

const keys = new Set<ImportSource>(['granola', 'folder']);
let progress: ImportProgress = {
  source: null,
  phase: 'idle',
  total: 0,
  imported: 0,
  skipped: 0,
  failed: 0,
  error: null,
};
let stopping = false;

export const events = {
  importProgressChanged: mockEvent<Generated.ImportProgressChanged>(
    'plugin:importers:import-progress-changed',
  ),
} satisfies typeof Generated.events;

function publish(next: ImportProgress): void {
  progress = next;
  void events.importProgressChanged.emit({ progress: next });
}

/** One meeting further, or the end. */
function tick(): void {
  if (progress.phase !== 'importing') return;
  if (stopping || progress.imported >= progress.total) {
    publish({ ...progress, phase: stopping ? 'cancelled' : 'finished' });
    return;
  }
  publish({ ...progress, imported: progress.imported + 1 });
  setTimeout(tick, TICK_MS);
}

const missingKey = (source: ImportSource) =>
  failure<never>({ code: 'not_found', message: `save a ${source} API key first` });

export const commands = {
  async sources() {
    await latency();
    return ok(
      SOURCES.map((source) => ({ source, key: keys.has(source) ? 'stored' : 'missing' }) as const),
    );
  },
  async saveKey(source: ImportSource, key: string) {
    await latency();
    if (key.trim() === '') return failure<null>({ code: 'invalid', message: 'the key is empty' });
    keys.add(source);
    return ok(null);
  },
  async forgetKey(source: ImportSource) {
    await latency();
    if (source !== 'folder') keys.delete(source);
    return ok(null);
  },
  async pickFolder() {
    await latency();
    return ok('/Users/sam/Documents/Otter export');
  },
  async preview(target: Generated.ImportTarget) {
    await latency();
    if (!keys.has(target.source)) return missingKey(target.source);
    const { total, already } = LIBRARY[target.source];
    return ok({ source: target.source, total, alreadyImported: already, new: total - already });
  },
  async start(target: Generated.ImportTarget) {
    await latency();
    if (!keys.has(target.source)) return missingKey(target.source);
    const { total, already } = LIBRARY[target.source];
    stopping = false;
    const first: ImportProgress = {
      ...progress,
      source: target.source,
      phase: 'importing',
      total: total - already,
      imported: 0,
      skipped: already,
      failed: 0,
      error: null,
    };
    publish(first);
    setTimeout(tick, TICK_MS);
    return ok(first);
  },
  async cancel() {
    await latency();
    stopping = progress.phase === 'importing';
    return ok(stopping);
  },
  async status() {
    await latency();
    return ok(progress);
  },
} satisfies typeof Generated.commands;
