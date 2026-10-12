import { useMutation, useQueryClient } from '@tanstack/react-query';

import { llmApi, llmKeys } from '@/features/llm';
import { modelSelectionPatch, secretsApi, settingsApi, settingsKeys } from '@/features/settings';

import { KEYED_PROVIDERS, type KeyedChoice, openAiPins } from '../ai-choice';

/** Input of {@link useConnectProvider}. */
export interface ConnectInput {
  choice: KeyedChoice;
  key: string;
}

/**
 * Checks a pasted key through the LLM gateway (listing the provider's
 * models with it, unsaved), then saves it in the Keychain, makes the
 * provider the notes provider and, for OpenAI, pins every generating route
 * to it. A rejected key saves nothing.
 */
async function connect({ choice, key }: ConnectInput): Promise<void> {
  const provider = KEYED_PROVIDERS[choice];
  await llmApi.models(provider.gateway, { apiKey: key });
  await secretsApi.set(provider.secret, key);
  await settingsApi.patch(modelSelectionPatch({ provider: provider.settings, model: '' }));
  if (choice !== 'openai') return;
  const pins = openAiPins(await llmApi.routes());
  await Promise.all(pins.map((pin) => llmApi.setRoute(pin.route, pin.target)));
}

/** Connects Claude or OpenAI with a pasted key; see {@link connect}. */
export function useConnectProvider() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: connect,
    onSuccess: () =>
      Promise.all([
        client.invalidateQueries({ queryKey: settingsKeys.all }),
        client.invalidateQueries({ queryKey: llmKeys.all }),
      ]),
  });
}
