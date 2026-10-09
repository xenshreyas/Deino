/**
 * Typed calls to the `llm` plugin (the LLM gateway's Settings → AI surface).
 * Each resolves to the data or throws an `IpcError`; components use the hooks
 * in `queries.ts`. No call ever returns an API key: providers report masked keys.
 */
import { Channel } from '@tauri-apps/api/core';

import { commands } from '@/lib/ipc/generated/llm';
import { unwrap } from '@/lib/ipc/result';

import type {
  DraftConnection,
  LlmStreamEvent,
  ModelInfo,
  ProviderId,
  ProviderStatus,
  ResolvedRoute,
  Route,
  RouteTestReport,
  Target,
} from './types';

export const llmApi = {
  /** Every provider with its masked key and endpoint. */
  providers: (): Promise<ProviderStatus[]> => unwrap(commands.providers()),
  /** The models `provider` can serve, reached with any unsaved form values. */
  models: (provider: ProviderId, draft: DraftConnection = {}): Promise<ModelInfo[]> =>
    unwrap(commands.models(provider, draft.apiKey ?? null, draft.endpoint ?? null)),
  /** Every task route and what serves it. */
  routes: (): Promise<ResolvedRoute[]> => unwrap(commands.routes()),
  /** Pins `route` to `target`, or resets it to its default with `null`. */
  setRoute: (route: Route, target: Target | null): Promise<ResolvedRoute> =>
    unwrap(commands.setRoute(route, target)),
  /** Sends a test message on `route`, calling `onText` with each piece of the reply. */
  testRoute: (route: Route, onText: (delta: string) => void): Promise<RouteTestReport> => {
    const channel = new Channel<LlmStreamEvent>();
    channel.onmessage = (event) => {
      onText(event.delta);
    };
    return unwrap(commands.testRoute(route, channel));
  },
  /** Downloads `model` onto the Ollama server; progress arrives as `ollamaPull` events. */
  ollamaPull: (model: string): Promise<null> => unwrap(commands.ollamaPull(model)),
  /** Deletes `model` from the Ollama server. */
  ollamaDelete: (model: string): Promise<null> => unwrap(commands.ollamaDelete(model)),
};
