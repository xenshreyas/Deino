import { describe, expect, test } from 'vitest';

import { providerOf } from './engine';

describe('the legacy provider names', () => {
  test('each engine has its own legacy name', () => {
    expect(providerOf('parakeet_unified')).toBe('parakeetUnified');
    expect(providerOf('parakeet')).toBe('parakeet');
    expect(providerOf('whisper')).toBe('localWhisper');
  });
});
