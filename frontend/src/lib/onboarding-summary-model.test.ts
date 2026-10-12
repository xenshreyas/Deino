import { describe, expect, test } from 'vitest';

import {
  getDownloadTotalMb,
  getSummaryModelSizeLabel,
  getSummaryModelSizeMb,
} from './onboarding-summary-model';

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
