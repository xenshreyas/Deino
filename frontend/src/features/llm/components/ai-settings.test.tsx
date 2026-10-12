import { QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, within } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { createQueryClient } from '@/lib/ipc';
import type * as Generated from '@/lib/ipc/generated/llm';

import { llmCopy } from '../copy';
import { AiSettings } from './ai-settings';

// Typed against the generated bindings: a Rust signature change breaks this mock.
const commands = vi.hoisted(
  () =>
    ({
      providers: vi.fn<typeof Generated.commands.providers>(),
      models: vi.fn<typeof Generated.commands.models>(),
      routes: vi.fn<typeof Generated.commands.routes>(),
      setRoute: vi.fn<typeof Generated.commands.setRoute>(),
      testRoute: vi.fn<typeof Generated.commands.testRoute>(),
      ollamaPull: vi.fn<typeof Generated.commands.ollamaPull>(),
      ollamaDelete: vi.fn<typeof Generated.commands.ollamaDelete>(),
    }) satisfies typeof Generated.commands,
);
vi.mock('@/lib/ipc/generated/llm', () => ({ commands }));

const SECRET_TAIL = 'secretvalue';

const providers: Generated.ProviderStatus[] = [
  { id: 'anthropic', name: 'Anthropic', requiresKey: true, maskedKey: 'sk-a…7f3c', endpoint: null },
  { id: 'openai', name: 'OpenAI', requiresKey: true, maskedKey: null, endpoint: null },
  {
    id: 'ollama',
    name: 'Ollama',
    requiresKey: false,
    maskedKey: null,
    endpoint: 'http://gpu-box:11434',
  },
];

const chat: Generated.ResolvedRoute = {
  route: 'chat',
  tier: 'smart',
  target: { provider: 'openai', model: 'gpt-5' },
  source: 'custom',
};
const title: Generated.ResolvedRoute = {
  route: 'title',
  tier: 'fast',
  target: { provider: 'anthropic', model: 'claude-haiku-5-5' },
  source: 'default',
};

/** Renders the section and opens its table, which starts closed. */
function renderSettings() {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <AiSettings />
    </QueryClientProvider>,
  );
  fireEvent.click(screen.getByRole('button', { name: llmCopy.showModels }));
}

describe('AiSettings', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    commands.providers.mockResolvedValue({ status: 'ok', data: providers });
    commands.routes.mockResolvedValue({ status: 'ok', data: [chat, title] });
    commands.models.mockResolvedValue({ status: 'ok', data: [] });
  });

  it('lists providers with masked keys and endpoints, never a key', async () => {
    renderSettings();

    expect(await screen.findByText('sk-a…7f3c')).toBeDefined();
    expect(screen.getByText('http://gpu-box:11434')).toBeDefined();
    expect(screen.getByText(llmCopy.keyMissing)).toBeDefined();
    expect(document.body.textContent).not.toContain(SECRET_TAIL);
  });

  it('streams a route test reply and reports the model and latency', async () => {
    commands.testRoute.mockImplementation((_route, channel) => {
      channel.onmessage({ kind: 'text', delta: 'Hello ' });
      channel.onmessage({ kind: 'text', delta: 'there.' });
      return Promise.resolve({
        status: 'ok',
        data: {
          target: title.target,
          latencyMs: 820,
          usage: { inputTokens: 30, outputTokens: 4 },
          dimensions: null,
        },
      });
    });
    renderSettings();
    const row = (await screen.findByText(llmCopy.routes.title)).closest('li');

    fireEvent.click(within(row ?? document.body).getByRole('button', { name: llmCopy.test }));

    const status = await within(row ?? document.body).findByRole('status');
    expect(status.textContent).toContain('Hello there.');
    expect(status.textContent).toContain(llmCopy.testReport('claude-haiku-5-5', 820));
    expect(commands.testRoute.mock.calls[0]?.[0]).toBe('title');
  });

  it('resets a custom route to its default', async () => {
    commands.setRoute.mockResolvedValue({
      status: 'ok',
      data: {
        ...chat,
        target: { provider: 'anthropic', model: 'claude-sonnet-5-5' },
        source: 'default',
      },
    });
    renderSettings();
    const row = (await screen.findByText(llmCopy.routes.chat)).closest('li');

    fireEvent.click(within(row ?? document.body).getByRole('button', { name: llmCopy.reset }));

    await vi.waitFor(() => {
      expect(commands.setRoute).toHaveBeenCalledWith('chat', null);
    });
    expect(screen.queryByRole('button', { name: llmCopy.reset })).toBeNull();
  });

  it('shows a failed test as an inline error', async () => {
    commands.testRoute.mockResolvedValue({
      status: 'error',
      error: { code: 'unavailable', message: 'no API key is set for OpenAI' },
    });
    renderSettings();
    const row = (await screen.findByText(llmCopy.routes.chat)).closest('li');

    fireEvent.click(within(row ?? document.body).getByRole('button', { name: llmCopy.test }));

    expect((await screen.findByRole('alert')).textContent).toBe('no API key is set for OpenAI');
  });
});
