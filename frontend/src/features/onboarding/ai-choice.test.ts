import { describe, expect, it } from 'vitest';

import type { ResolvedRoute } from '@/features/llm';
import type { Settings } from '@/features/settings';

import { initialChoice, isKeyed, openAiPins } from './ai-choice';

const AI: Settings['ai'] = {
  provider: 'claude',
  model: '',
  providerModels: {},
  ollamaEndpoint: null,
  customOpenai: null,
  autoSummary: false,
  routes: {},
};

const route = (overrides: Partial<ResolvedRoute>): ResolvedRoute => ({
  route: 'notes.enhance',
  tier: 'smart',
  target: { provider: 'anthropic', model: 'claude-sonnet-5-5' },
  source: 'default',
  ...overrides,
});

describe('pinning routes to OpenAI', () => {
  it('sends smart routes to the large model and fast ones to the small one', () => {
    const pins = openAiPins([route({}), route({ route: 'tagging', tier: 'fast' })]);

    expect(pins).toEqual([
      { route: 'notes.enhance', target: { provider: 'openai', model: 'gpt-5' } },
      { route: 'tagging', target: { provider: 'openai', model: 'gpt-5-mini' } },
    ]);
  });

  it('keeps embeddings on this Mac', () => {
    expect(openAiPins([route({ route: 'embed', tier: 'embedding' })])).toEqual([]);
  });
});

describe('the first choice shown', () => {
  it('follows the saved provider', () => {
    expect(initialChoice({ ...AI, provider: 'openai' })).toBe('openai');
    expect(initialChoice({ ...AI, provider: 'builtin-ai' })).toBe('local');
  });

  it('is Claude for any other provider', () => {
    expect(initialChoice({ ...AI, provider: 'ollama' })).toBe('claude');
  });

  it('knows which choices need a key', () => {
    expect(isKeyed('claude')).toBe(true);
    expect(isKeyed('local')).toBe(false);
  });
});
