import { QueryClientProvider } from '@tanstack/react-query';
import { act, renderHook } from '@testing-library/react';
import type { ReactNode } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import type * as Secrets from '@/lib/ipc/generated/secrets';
import type * as Generated from '@/lib/ipc/generated/settings';
import { createQueryClient } from '@/lib/ipc';

import { modelSelectionPatch, typedKeyOf, useSaveModelSelection } from './model-selection';
import { defaultSettings } from './settings-fixture';

const bindings = vi.hoisted(() => {
  const calls: string[] = [];
  const settings = {
    get: vi.fn<typeof Generated.commands.get>(),
    patch: vi.fn<typeof Generated.commands.patch>(),
  } satisfies typeof Generated.commands;
  const secrets = {
    set: vi.fn<typeof Secrets.commands.set>(),
    status: vi.fn<typeof Secrets.commands.status>(),
    delete: vi.fn<typeof Secrets.commands.delete>(),
  } satisfies typeof Secrets.commands;
  return { calls, settings, secrets };
});
vi.mock('@/lib/ipc/generated/settings', () => ({ commands: bindings.settings, events: {} }));
vi.mock('@/lib/ipc/generated/secrets', () => ({ commands: bindings.secrets }));

describe('modelSelectionPatch', () => {
  it('saves the provider, its model and the model remembered for it', () => {
    const patch = modelSelectionPatch({ provider: 'claude', model: 'claude-sonnet-4-5' });

    expect(patch).toEqual({
      ai: {
        provider: 'claude',
        model: 'claude-sonnet-4-5',
        providerModels: { claude: 'claude-sonnet-4-5' },
      },
    });
  });

  it('saves the Ollama endpoint only for Ollama, and never a key', () => {
    const ollama = modelSelectionPatch({
      provider: 'ollama',
      model: 'llama3.2',
      ollamaEndpoint: 'http://mac-mini:11434',
      typedKey: 'should-not-appear',
    });
    const openai = modelSelectionPatch({
      provider: 'openai',
      model: 'gpt-4o',
      ollamaEndpoint: 'x',
    });

    expect(ollama.ai?.ollamaEndpoint).toBe('http://mac-mini:11434');
    expect(JSON.stringify(ollama)).not.toContain('should-not-appear');
    expect(openai.ai).not.toHaveProperty('ollamaEndpoint');
  });
});

describe('typedKeyOf', () => {
  it('names the provider’s secret for a typed key and ignores blanks and keyless providers', () => {
    expect(typedKeyOf({ provider: 'openai', model: '', typedKey: ' sk-1 ' })).toEqual({
      name: 'openai_api_key',
      value: 'sk-1',
    });
    expect(typedKeyOf({ provider: 'openai', model: '', typedKey: '  ' })).toBeNull();
    expect(typedKeyOf({ provider: 'builtin-ai', model: '', typedKey: 'x' })).toBeNull();
  });
});

describe('useSaveModelSelection', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    bindings.calls.length = 0;
    bindings.secrets.set.mockImplementation(() => {
      bindings.calls.push('secret');
      return Promise.resolve({ status: 'ok', data: null });
    });
    bindings.settings.patch.mockImplementation(() => {
      bindings.calls.push('patch');
      return Promise.resolve({ status: 'ok', data: defaultSettings() });
    });
  });

  it('saves a typed key to the Keychain before the settings', async () => {
    const client = createQueryClient();
    const wrapper = ({ children }: { children: ReactNode }) => (
      <QueryClientProvider client={client}>{children}</QueryClientProvider>
    );
    const { result } = renderHook(useSaveModelSelection, { wrapper });

    await act(() =>
      result.current.mutateAsync({ provider: 'groq', model: 'llama-3.3', typedKey: 'gsk-1' }),
    );

    expect(bindings.calls).toEqual(['secret', 'patch']);
    expect(bindings.secrets.set).toHaveBeenCalledWith('groq_api_key', 'gsk-1');
  });
});
