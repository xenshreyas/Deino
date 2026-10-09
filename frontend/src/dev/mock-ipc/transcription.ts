/**
 * Mock of `@/lib/ipc/generated/transcription`: the six Parakeet Unified
 * quants in memory, `Q8_0` already downloaded, downloads that finish after
 * a few progress events, and the model events the UI listens to.
 */
import type * as Generated from '@/lib/ipc/generated/transcription';
import type { CommandResult } from '@/lib/ipc/result';

import { latency } from './latency';
import { failure, ok } from './command-result';
import { mockEvent } from './mock-event';

const NAME = 'Parakeet Unified EN 0.6B';

/** Progress events a mock download sends before it completes. */
const PROGRESS_STEPS = 4;

const SIZES: [Generated.Quant, number][] = [
  ['Q4_K_M', 477_274_496],
  ['Q5_K_M', 540_795_264],
  ['Q6_K', 602_191_232],
  ['Q8_0', 731_357_568],
  ['F16', 1_239_114_240],
  ['F32', 2_473_323_520],
];

function fileId(quant: Generated.Quant): Generated.ModelFileId {
  return `parakeet-unified-en-0.6b-${quant}` as Generated.ModelFileId;
}

const files: Generated.ModelFileView[] = SIZES.map(([quant, sizeBytes]) => ({
  id: fileId(quant),
  name: NAME,
  quant,
  sizeBytes,
  state: quant === 'Q8_0' ? { kind: 'ready' } : { kind: 'missing' },
  recommended: quant === 'Q8_0',
}));

export const events = {
  liveTranscript: mockEvent<Generated.LiveTranscript>('plugin:transcription:live-transcript'),
  modelChanged: mockEvent<Generated.ModelChanged>('plugin:transcription:model-changed'),
  modelDownloadProgress: mockEvent<Generated.ModelDownloadProgress>(
    'plugin:transcription:model-download-progress',
  ),
} satisfies typeof Generated.events;

function find(id: Generated.ModelFileId): Generated.ModelFileView | undefined {
  return files.find((file) => file.id === id);
}

function unknown<T>(id: Generated.ModelFileId): CommandResult<T> {
  return failure<T>({ code: 'not_found', message: `unknown model ${id}` });
}

async function download(file: Generated.ModelFileView): Promise<void> {
  for (let step = 1; step <= PROGRESS_STEPS; step += 1) {
    const downloadedBytes = Math.round((file.sizeBytes * step) / PROGRESS_STEPS);
    file.state = { kind: 'downloading', downloadedBytes };
    await events.modelDownloadProgress.emit({
      progress: { id: file.id, downloadedBytes, totalBytes: file.sizeBytes },
    });
    await latency();
  }
  file.state = { kind: 'ready' };
  await events.modelChanged.emit({ id: file.id });
}

export const commands = {
  async listModels() {
    await latency();
    return ok(files.map((file) => ({ ...file })));
  },
  async downloadModel(id) {
    const file = find(id);
    if (!file) return unknown<null>(id);
    await download(file);
    return ok(null);
  },
  async cancelDownload(id) {
    await latency();
    return ok(find(id)?.state.kind === 'downloading');
  },
  async deleteModel(id) {
    await latency();
    const file = find(id);
    if (!file) return unknown<null>(id);
    file.state = { kind: 'missing' };
    await events.modelChanged.emit({ id });
    return ok(null);
  },
} satisfies typeof Generated.commands;
