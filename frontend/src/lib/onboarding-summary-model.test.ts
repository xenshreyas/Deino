import { describe, expect, test } from 'vitest';

import {
  getDownloadTotalMb,
  getSummaryModelSizeLabel,
} from './onboarding-summary-model';

describe('summary model sizes', () => {
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
