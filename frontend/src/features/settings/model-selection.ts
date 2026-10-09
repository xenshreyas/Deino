/**
 * Saving the summary model the user picked: one settings patch plus, when
 * the user typed a key, one Keychain write.
 *
 * The model picker (`components/ModelSettingsModal.tsx`, legacy until
 * FND-09/FND-10 rebuild it) edits a draft that may hold a freshly typed
 * key; this module turns that draft into the two writes.
 */
import { useMutation, useQueryClient } from '@tanstack/react-query';

import { secretsApi, settingsApi } from './api';
import { settingsKeys } from './queries';
import type { CustomOpenAiSettings, LlmProvider, SecretName, SettingsPatch } from './types';

/** The secret each provider's key is saved under; the built-in model has none. */
export const PROVIDER_KEYS = {
  'builtin-ai': null,
  openai: 'openai_api_key',
  claude: 'anthropic_api_key',
  groq: 'groq_api_key',
  openrouter: 'openrouter_api_key',
  ollama: 'ollama_api_key',
  'custom-openai': 'custom_openai_api_key',
} as const satisfies Record<LlmProvider, SecretName | null>;

/** A model choice to save. */
export interface ModelSelection {
  provider: LlmProvider;
  model: string;
  /** The Ollama server; `null` for the local default. Saved only for Ollama. */
  ollamaEndpoint?: string | null;
  /** The custom endpoint. Saved only for the custom provider. */
  customOpenai?: CustomOpenAiSettings | null;
  /** A key the user just typed; blank or absent keeps the saved one. */
  typedKey?: string | null;
}

/** The settings patch that saves `selection` (never its key). */
export function modelSelectionPatch(selection: ModelSelection): SettingsPatch {
  const { provider, model } = selection;
  return {
    ai: {
      provider,
      model,
      providerModels: model.trim() === '' ? {} : { [provider]: model },
      ...(provider === 'ollama' ? { ollamaEndpoint: selection.ollamaEndpoint ?? null } : {}),
      ...(provider === 'custom-openai' && selection.customOpenai
        ? { customOpenai: selection.customOpenai }
        : {}),
    },
  };
}

/** The key to save with `selection`, if the user typed one. */
export function typedKeyOf(selection: ModelSelection): { name: SecretName; value: string } | null {
  const name = PROVIDER_KEYS[selection.provider];
  const value = selection.typedKey?.trim() ?? '';
  return name === null || value === '' ? null : { name, value };
}

/**
 * Saves a model choice: the key first, so a rejected key leaves the old
 * choice in place, then the settings.
 */
export function useSaveModelSelection() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: async (selection: ModelSelection) => {
      const key = typedKeyOf(selection);
      if (key) await secretsApi.set(key.name, key.value);
      return settingsApi.patch(modelSelectionPatch(selection));
    },
    onSuccess: (settings, selection) => {
      client.setQueryData(settingsKeys.document(), settings);
      const key = typedKeyOf(selection);
      if (key) client.setQueryData(settingsKeys.secret(key.name), 'stored');
    },
  });
}
