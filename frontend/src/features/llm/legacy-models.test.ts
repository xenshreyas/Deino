import { beforeEach, describe, expect, it, vi } from 'vitest';

import type * as Generated from '@/lib/ipc/generated/llm';

import { formatBytes, legacyModelIds, legacyOllamaModels } from './legacy-models';

const models = vi.hoisted(() => vi.fn<typeof Generated.commands.models>());
vi.mock('@/lib/ipc/generated/llm', () => ({ commands: { models } }));

const gemma: Generated.ModelInfo = {
  id: 'gemma3:1b',
  displayName: null,
  contextLength: null,
  sizeBytes: 815_319_791,
  modifiedAt: '2026-10-01T10:00:00Z',
  ready: true,
};

describe('legacy model lists', () => {
  beforeEach(() => {
    models.mockReset();
    models.mockResolvedValue({ status: 'ok', data: [gemma] });
  });

  it('formats sizes like the legacy Ollama command', () => {
    expect(formatBytes(512)).toBe('512 B');
    expect(formatBytes(815_319_791)).toBe('777.5 MB');
    expect(formatBytes(4 * 1024 ** 3)).toBe('4.0 GB');
  });

  it('serves Ollama models in the legacy shape from a typed endpoint', async () => {
    const listed = await legacyOllamaModels('http://gpu-box:11434');

    expect(listed).toEqual([
      { name: 'gemma3:1b', id: 'gemma3:1b', size: '777.5 MB', modified: '2026-10-01T10:00:00Z' },
    ]);
    expect(models).toHaveBeenCalledWith('ollama', null, 'http://gpu-box:11434');
  });

  it('lists a hosted provider with a typed, unsaved key', async () => {
    expect(await legacyModelIds('openai', { apiKey: 'sk-typed' })).toEqual(['gemma3:1b']);
    expect(models).toHaveBeenCalledWith('openai', 'sk-typed', null);
  });
});
