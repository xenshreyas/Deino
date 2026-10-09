import { describe, expect, test } from 'vitest';

import {
  getDownloadTotalMb,
  getSummaryModelSizeLabel,
  getSummaryModelSizeMb,
  resolveOnboardingSummaryModelStatus,
} from './onboarding-summary-model';

describe('onboarding summary model status', () => {
  test('legacy Gemma availability does not make an undownloaded selected Qwen model ready', () => {
    expect(
      resolveOnboardingSummaryModelStatus({
        selectedModel: 'qwen3.5:4b',
        recommendedModel: 'qwen3.5:4b',
        selectedModelReady: false,
      }),
    ).toEqual({ selectedSummaryModel: 'qwen3.5:4b', summaryModelDownloaded: false });
  });

  test('an explicitly selected model wins over a different recommendation', () => {
    expect(
      resolveOnboardingSummaryModelStatus({
        selectedModel: 'gemma3:1b',
        recommendedModel: 'qwen3.5:4b',
        selectedModelReady: true,
      }),
    ).toEqual({ selectedSummaryModel: 'gemma3:1b', summaryModelDownloaded: true });
  });

  test('the recommended Qwen becomes the selected model when none is selected yet', () => {
    expect(
      resolveOnboardingSummaryModelStatus({
        selectedModel: '',
        recommendedModel: 'qwen3.5:2b',
        selectedModelReady: true,
      }),
    ).toEqual({ selectedSummaryModel: 'qwen3.5:2b', summaryModelDownloaded: true });
  });
});

describe('summary model sizes', () => {
  test('known models report their download size in MB', () => {
    expect(getSummaryModelSizeMb('qwen3.5:2b')).toBe(1221);
    expect(getSummaryModelSizeMb('qwen3.5:4b')).toBe(2614);
    expect(getSummaryModelSizeMb('gemma3:1b')).toBe(1019);
    expect(getSummaryModelSizeMb('unknown:model')).toBe(0);
  });

  test('size labels round to GiB and stay empty for unknown models', () => {
    expect(getSummaryModelSizeLabel('qwen3.5:2b')).toBe('~1.2 GiB');
    expect(getSummaryModelSizeLabel('qwen3.5:4b')).toBe('~2.6 GiB');
    expect(getSummaryModelSizeLabel('unknown:model')).toBe('');
  });

  test('a reported download total wins over the known model size', () => {
    expect(getDownloadTotalMb(0, 'qwen3.5:4b')).toBe(2614);
    expect(getDownloadTotalMb(undefined, 'qwen3.5:2b')).toBe(1221);
    expect(getDownloadTotalMb(512, 'qwen3.5:4b')).toBe(512);
  });
});
