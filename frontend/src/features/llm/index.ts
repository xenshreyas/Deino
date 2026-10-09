/**
 * The LLM feature: Settings → AI over the gateway's `llm` plugin. Providers
 * with masked keys, each provider's models, the per-route model table with a
 * streamed Test per route, and Ollama model downloads (`ollamaPull` events).
 */
export { llmApi } from './api';
export { AiSettings } from './components/ai-settings';
export { llmCopy } from './copy';
export {
  formatBytes,
  type LegacyOllamaModel,
  type LegacyOpenRouterModel,
  legacyModelIds,
  legacyOllamaModels,
  legacyOpenRouterModels,
} from './legacy-models';
export {
  llmKeys,
  modelsQuery,
  providersQuery,
  routesQuery,
  useLlmSettingsChanges,
  useModels,
  useProviders,
  useRoutePin,
  useRoutes,
  useSetRoute,
  useTestRoute,
} from './queries';
export type {
  DraftConnection,
  ModelInfo,
  OllamaPull,
  OllamaPullStatus,
  ProviderId,
  ProviderStatus,
  ResolvedRoute,
  Route,
  RouteTestReport,
  Target,
} from './types';
