import { describe, expect, it } from 'vitest';

import { activeModel, downloadedBytes, isReady } from './models';
import type { ModelFileId, ModelFileView, ModelState, Quant } from './types';

function file(quant: Quant, state: ModelState, recommended = false): ModelFileView {
  return {
    id: `parakeet-unified-en-0.6b-${quant}` as ModelFileId,
    name: 'Parakeet Unified EN 0.6B',
    quant,
    sizeBytes: 1000,
    state,
    recommended,
  };
}

const q5 = file('Q5_K_M', { kind: 'ready' });
const q8 = file('Q8_0', { kind: 'missing' }, true);

describe('activeModel', () => {
  it('uses the selected file when it exists', () => {
    expect(activeModel([q5, q8], q5.id)).toBe(q5);
  });

  it('falls back to the recommended file when nothing or an unknown file is selected', () => {
    expect(activeModel([q5, q8], undefined)).toBe(q8);
    expect(activeModel([q5, q8], 'parakeet-tdt-0.6b-v3-int8')).toBe(q8);
  });
});

describe('isReady and downloadedBytes', () => {
  it('only a verified file is ready', () => {
    expect(isReady(q5)).toBe(true);
    expect(isReady(q8)).toBe(false);
    expect(isReady(undefined)).toBe(false);
  });

  it('reports the bytes on disk of a running or paused download', () => {
    expect(downloadedBytes(file('Q6_K', { kind: 'partial', downloadedBytes: 300 }))).toBe(300);
    expect(downloadedBytes(file('Q6_K', { kind: 'downloading', downloadedBytes: 40 }))).toBe(40);
    expect(downloadedBytes(q5)).toBeNull();
  });
});
