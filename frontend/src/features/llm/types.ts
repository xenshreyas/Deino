/** The LLM feature's types, generated from Rust by `bindings.rs`. */
export type {
  LlmStreamEvent,
  ModelInfo,
  OllamaPull,
  OllamaPullStatus,
  ProviderId,
  ProviderStatus,
  ResolvedRoute,
  Route,
  RouteSource,
  RouteTestReport,
  RouteTier,
  Target,
} from '@/lib/ipc/generated/llm';

/** Values typed into a settings form but not saved yet. */
export interface DraftConnection {
  /** A key the user typed. */
  apiKey?: string | null;
  /** A server address the user typed (Ollama and custom endpoints). */
  endpoint?: string | null;
}
