import { describe, expect, it } from 'vitest';

import { engineOf, providerOf } from './engine';

describe('engine bridge', () => {
  it('maps every settings engine to its legacy provider and back', () => {
    for (const engine of ['parakeet_unified', 'parakeet', 'whisper'] as const) {
      expect(engineOf(providerOf(engine))).toBe(engine);
    }
  });

  it('names Parakeet Unified the way the backend expects', () => {
    expect(providerOf('parakeet_unified')).toBe('parakeetUnified');
    expect(providerOf('whisper')).toBe('localWhisper');
  });

  it('falls back to the default engine for unknown providers', () => {
    expect(engineOf('somethingElse')).toBe('parakeet_unified');
  });
});
