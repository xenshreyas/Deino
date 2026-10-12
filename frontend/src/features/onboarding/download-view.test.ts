import { describe, expect, it } from 'vitest';

import type { ModelFileId, ModelFileView } from '@/features/transcription';

import { downloadView, percentOf, sizeLabel } from './download-view';

const model = (state: ModelFileView['state']): ModelFileView => ({
  id: 'parakeet-unified-en-0.6b-Q5_K_M' as ModelFileId,
  name: 'Parakeet Unified EN',
  quant: 'Q5_K_M',
  sizeBytes: 1_000,
  state,
  recommended: true,
});

describe('a model download as a row', () => {
  it('is checking until the list loads', () => {
    expect(downloadView(undefined)).toEqual({ kind: 'checking' });
  });

  it('offers the download when the file is missing, partial or unknown', () => {
    expect(downloadView(null)).toEqual({ kind: 'missing' });
    expect(downloadView(model({ kind: 'missing' }))).toEqual({ kind: 'missing' });
    expect(downloadView(model({ kind: 'partial', downloadedBytes: 400 }))).toEqual({
      kind: 'missing',
    });
  });

  it('shows progress, preferring the newer of the event and the listing', () => {
    const downloading = model({ kind: 'downloading', downloadedBytes: 250 });

    expect(downloadView(downloading)).toEqual({ kind: 'downloading', percent: 25 });
    expect(downloadView(downloading, 600)).toEqual({ kind: 'downloading', percent: 60 });
    expect(downloadView(downloading, 100)).toEqual({ kind: 'downloading', percent: 25 });
  });

  it('shows a download the UI started before the listing catches up', () => {
    expect(downloadView(model({ kind: 'missing' }), 100, true)).toEqual({
      kind: 'downloading',
      percent: 10,
    });
  });

  it('is ready once verified', () => {
    expect(downloadView(model({ kind: 'ready' }))).toEqual({ kind: 'ready' });
  });
});

describe('percentages and sizes', () => {
  it('clamp to 0–100 and survive an unknown total', () => {
    expect(percentOf(5, 0)).toBe(0);
    expect(percentOf(2_000, 1_000)).toBe(100);
    expect(percentOf(999, 1_000)).toBe(99);
  });

  it('label sizes in whole megabytes', () => {
    expect(sizeLabel(139_400_000)).toBe('139 MB');
  });
});
