/**
 * The model lists in the shapes the inherited model pickers expect
 * (`ModelSettingsModal`, `ConfigContext`, the meeting page), served by the
 * `llm` plugin since FND-09 replaced the per-provider legacy commands. These
 * adapters go away when NOT-04 and the settings rewrite port those pickers.
 */
import { llmApi } from './api';
import type { DraftConnection, ModelInfo, ProviderId } from './types';

/** An Ollama model as the inherited pickers show it. */
export interface LegacyOllamaModel {
  name: string;
  id: string;
  size: string;
  modified: string;
}

/** An OpenRouter model as the inherited picker shows it. */
export interface LegacyOpenRouterModel {
  id: string;
  name: string;
  context_length?: number;
}

const UNITS = ['B', 'KB', 'MB', 'GB'] as const;
const KIBI = 1024;

/** `815319791` → `777.5 MB`, like the legacy command formatted sizes. */
export function formatBytes(bytes: number): string {
  let value = bytes;
  let unit = 0;
  while (value >= KIBI && unit < UNITS.length - 1) {
    value /= KIBI;
    unit += 1;
  }
  return unit === 0 ? `${String(value)} B` : `${value.toFixed(1)} ${UNITS[unit] ?? 'GB'}`;
}

function toOllama(model: ModelInfo): LegacyOllamaModel {
  return {
    name: model.id,
    id: model.id,
    size: model.sizeBytes === null ? '' : formatBytes(model.sizeBytes),
    modified: model.modifiedAt ?? '',
  };
}

function toOpenRouter(model: ModelInfo): LegacyOpenRouterModel {
  const name = model.displayName ?? model.id;
  return model.contextLength === null
    ? { id: model.id, name }
    : { id: model.id, name, context_length: model.contextLength };
}

/** The Ollama server's models; `endpoint` is an address typed but not saved. */
export async function legacyOllamaModels(
  endpoint: string | null = null,
): Promise<LegacyOllamaModel[]> {
  return (await llmApi.models('ollama', { endpoint })).map(toOllama);
}

/** OpenRouter's models (listed without a key). */
export async function legacyOpenRouterModels(): Promise<LegacyOpenRouterModel[]> {
  return (await llmApi.models('openrouter')).map(toOpenRouter);
}

/** A hosted provider's model IDs, with a key typed but not saved. */
export async function legacyModelIds(
  provider: ProviderId,
  draft: DraftConnection = {},
): Promise<string[]> {
  return (await llmApi.models(provider, draft)).map((model) => model.id);
}
