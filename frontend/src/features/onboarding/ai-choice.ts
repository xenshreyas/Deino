/**
 * Who writes the notes, as onboarding offers it: Claude or OpenAI with the
 * user's key, or the built-in model on this Mac (§8 D3). Pure rules; the AI
 * step and its hook apply them through the settings, secrets and llm APIs.
 */
import type { ProviderId, ResolvedRoute, Route, Target } from '@/features/llm';
import type { LlmProvider, SecretName, Settings } from '@/features/settings';

/** The three choices of the notes picker. */
export type NotesChoice = 'claude' | 'openai' | 'local';

/** A cloud provider the user brings a key for. */
export type KeyedChoice = Exclude<NotesChoice, 'local'>;

/** How a keyed choice is named in the gateway, the settings and the Keychain. */
export interface KeyedProvider {
  gateway: ProviderId;
  settings: LlmProvider;
  secret: SecretName;
}

export const KEYED_PROVIDERS: Record<KeyedChoice, KeyedProvider> = {
  claude: { gateway: 'anthropic', settings: 'claude', secret: 'anthropic_api_key' },
  openai: { gateway: 'openai', settings: 'openai', secret: 'openai_api_key' },
};

/**
 * The OpenAI models onboarding pins the routes to: the first entries of
 * the gateway's OpenAI list (`deino-llm` `openai_compat/flavor.rs`). Claude
 * needs no pins, because the router already prefers it once its key exists.
 */
export const OPENAI_MODELS = { smart: 'gpt-5', fast: 'gpt-5-mini' } as const;

/** A route pin to save. */
export interface RoutePin {
  route: Route;
  target: Target;
}

/** Every generating route on OpenAI; embeddings stay local (§8 D3). */
export function openAiPins(routes: readonly ResolvedRoute[]): RoutePin[] {
  return routes.flatMap(({ route, tier }) =>
    tier === 'embedding'
      ? []
      : [{ route, target: { provider: 'openai', model: OPENAI_MODELS[tier] } }],
  );
}

/** The choice to show first: the saved provider when it is one of the three. */
export function initialChoice(ai: Settings['ai']): NotesChoice {
  switch (ai.provider) {
    case 'openai':
      return 'openai';
    case 'builtin-ai':
      return 'local';
    default:
      return 'claude';
  }
}

/** Whether `choice` needs a key. */
export function isKeyed(choice: NotesChoice): choice is KeyedChoice {
  return choice !== 'local';
}
