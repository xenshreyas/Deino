/**
 * Mock of `@/lib/ipc/generated/llm`: the gateway's Settings → AI surface in
 * memory. Claude's key is "saved" (matching the secrets mock), so text routes
 * default to Claude; a route test streams a canned reply over the channel.
 */
import type * as Generated from '@/lib/ipc/generated/llm';

import { latency } from './latency';
import { failure, ok } from './mock-database';
import { mockEvent } from './mock-event';

const PROVIDERS: Generated.ProviderStatus[] = [
  { id: 'anthropic', name: 'Anthropic', requiresKey: true, maskedKey: 'sk-a…7f3c', endpoint: null },
  { id: 'openai', name: 'OpenAI', requiresKey: true, maskedKey: null, endpoint: null },
  { id: 'groq', name: 'Groq', requiresKey: true, maskedKey: null, endpoint: null },
  { id: 'openrouter', name: 'OpenRouter', requiresKey: true, maskedKey: null, endpoint: null },
  {
    id: 'custom_openai',
    name: 'Custom OpenAI-compatible',
    requiresKey: false,
    maskedKey: null,
    endpoint: null,
  },
  { id: 'ollama', name: 'Ollama', requiresKey: false, maskedKey: null, endpoint: null },
  { id: 'builtin', name: 'Built-in', requiresKey: false, maskedKey: null, endpoint: null },
];

const MODELS: Partial<Record<Generated.ProviderId, string[]>> = {
  anthropic: ['claude-sonnet-5-5', 'claude-haiku-5-5'],
  openai: ['gpt-5', 'gpt-5-mini', 'gpt-4.1'],
  groq: ['llama-3.3-70b-versatile'],
  ollama: ['gemma3:1b', 'qwen3.5:4b'],
  builtin: ['qwen3.5:2b', 'qwen3.5:4b', 'gemma3:4b', 'gemma3:1b'],
};

const TIERS: Record<Generated.Route, Generated.RouteTier> = {
  'notes.enhance': 'smart',
  'notes.edit': 'smart',
  chat: 'smart',
  'extract.action_items': 'fast',
  'extract.insights': 'fast',
  tagging: 'fast',
  title: 'fast',
  'entity.summary': 'fast',
  brief: 'smart',
  'followup.email': 'smart',
  recap: 'smart',
  embed: 'embedding',
};

const pinned = new Map<Generated.Route, Generated.Target>();

function defaultTarget(tier: Generated.RouteTier): Generated.Target {
  if (tier === 'embedding') return { provider: 'builtin', model: 'nomic-embed-text-v1.5' };
  return {
    provider: 'anthropic',
    model: tier === 'smart' ? 'claude-sonnet-5-5' : 'claude-haiku-5-5',
  };
}

function resolve(route: Generated.Route): Generated.ResolvedRoute {
  const tier = TIERS[route];
  const custom = pinned.get(route);
  return custom === undefined
    ? { route, tier, target: defaultTarget(tier), source: 'default' }
    : { route, tier, target: custom, source: 'custom' };
}

function modelInfo(id: string): Generated.ModelInfo {
  return {
    id,
    displayName: null,
    contextLength: null,
    sizeBytes: null,
    modifiedAt: null,
    ready: true,
  };
}

export const events = {
  ollamaPull: mockEvent<Generated.OllamaPull>('plugin:llm:ollama-pull'),
} satisfies typeof Generated.events;

export const commands = {
  async providers() {
    await latency();
    return ok(structuredClone(PROVIDERS));
  },
  async models(provider) {
    await latency();
    return ok((MODELS[provider] ?? []).map(modelInfo));
  },
  async routes() {
    await latency();
    return ok((Object.keys(TIERS) as Generated.Route[]).map(resolve));
  },
  async setRoute(route, target) {
    await latency();
    if (route === 'embed' && target !== null) {
      return failure({
        code: 'invalid',
        message: 'Built-in does not support choosing the embedding model',
      });
    }
    if (target === null) pinned.delete(route);
    else pinned.set(route, target);
    return ok(resolve(route));
  },
  async testRoute(route, onEvent) {
    await latency();
    const resolved = resolve(route);
    if (route === 'embed') {
      return ok({
        target: resolved.target,
        latencyMs: 40,
        usage: { inputTokens: 0, outputTokens: 0 },
        dimensions: 768,
      });
    }
    for (const delta of ['Hello! ', 'I received ', 'your test message.']) {
      onEvent.onmessage({ kind: 'text', delta });
    }
    return ok({
      target: resolved.target,
      latencyMs: 820,
      usage: { inputTokens: 31, outputTokens: 9 },
      dimensions: null,
    });
  },
  async ollamaPull(model) {
    for (const percent of [25, 60, 100]) {
      await latency();
      void events.ollamaPull.emit({ model, status: { kind: 'progress', percent } });
    }
    void events.ollamaPull.emit({ model, status: { kind: 'done' } });
    return ok(null);
  },
  async ollamaDelete() {
    await latency();
    return ok(null);
  },
} satisfies typeof Generated.commands;
