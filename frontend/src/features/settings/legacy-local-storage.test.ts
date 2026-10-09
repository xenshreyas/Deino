import { describe, expect, it, vi } from 'vitest';

import {
  LEGACY_KEYS,
  migrateLegacyLocalStorage,
  readLegacyLocalStorage,
} from './legacy-local-storage';

/** A `Storage`-shaped map, so tests never touch the real localStorage. */
function storage(entries: Record<string, string>) {
  const values = new Map(Object.entries(entries));
  return {
    getItem: (key: string) => values.get(key) ?? null,
    removeItem: (key: string) => {
      values.delete(key);
    },
    keys: () => [...values.keys()],
  };
}

describe('readLegacyLocalStorage', () => {
  it('turns every legacy key into one settings patch', () => {
    const legacy = storage({
      isAutoSummary: 'true',
      showConfidenceIndicator: 'false',
      providerModelMap: JSON.stringify({ openai: 'gpt-4o', claude: 'claude-sonnet-4-5' }),
      betaFeatures: JSON.stringify({ importAndRetranscribe: false }),
      'deino.meetingDetails.transcriptPaneRatio': '0.42',
    });

    const { patch } = readLegacyLocalStorage(legacy);

    expect(patch).toEqual({
      ai: { autoSummary: true, providerModels: { openai: 'gpt-4o', claude: 'claude-sonnet-4-5' } },
      ui: { showConfidenceIndicator: false, transcriptPaneRatio: 0.42 },
      beta: { importAndRetranscribe: false },
    });
  });

  it('drops values today’s settings would reject, but still lists their keys', () => {
    const legacy = storage({
      isAutoSummary: 'maybe',
      providerModelMap: JSON.stringify({ gemini: 'pro', openai: '' }),
      betaFeatures: '{ broken',
      'deino.meetingDetails.transcriptPaneRatio': '0.9',
      primaryLanguage: 'fr',
    });

    const { patch, keys } = readLegacyLocalStorage(legacy);

    expect(patch).toEqual({});
    expect(keys).toHaveLength(5);
  });
});

describe('migrateLegacyLocalStorage', () => {
  it('applies the patch, then deletes every legacy key', async () => {
    const legacy = storage({ isAutoSummary: 'true', primaryLanguage: 'en', unrelated: 'kept' });
    const apply = vi.fn(() => Promise.resolve());

    await migrateLegacyLocalStorage(legacy, apply);

    expect(apply).toHaveBeenCalledWith({ ai: { autoSummary: true } });
    expect(legacy.keys()).toEqual(['unrelated']);
  });

  it('keeps the keys for the next launch when saving fails', async () => {
    const legacy = storage({ isAutoSummary: 'true' });

    const failed = migrateLegacyLocalStorage(legacy, () => Promise.reject(new Error('disk full')));

    await expect(failed).rejects.toThrow('disk full');
    expect(legacy.keys()).toEqual([LEGACY_KEYS.autoSummary]);
  });

  it('does nothing once the keys are gone', async () => {
    const apply = vi.fn(() => Promise.resolve());

    await migrateLegacyLocalStorage(storage({}), apply);

    expect(apply).not.toHaveBeenCalled();
  });
});
