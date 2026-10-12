import { QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import type * as Generated from '@/lib/ipc/generated/settings';
import { DEFAULT_SETTINGS } from '@/lib/ipc/generated/settings-defaults';
import { createQueryClient } from '@/lib/ipc';

import { aiProviderCopy as copy } from '../copy';
import { AiProviderSettings } from './ai-provider-settings';

const bindings = vi.hoisted(() => ({
  get: vi.fn<typeof Generated.commands.get>(),
  patch: vi.fn<typeof Generated.commands.patch>(),
  listen: vi.fn(() => Promise.resolve(vi.fn())),
}));
vi.mock('@/lib/ipc/generated/settings', () => ({
  commands: { get: bindings.get, patch: bindings.patch },
  events: { settingsChanged: { listen: bindings.listen } },
}));
vi.mock('@/lib/ipc/generated/llm', () => ({ commands: {}, events: {} }));
// The picker and the inherited panel have their own tests; only the section is under test.
vi.mock('@/features/onboarding', () => ({ NotesProvider: () => <p>Notes provider picker</p> }));
vi.mock('@/components/SummaryModelSettings', () => ({
  OtherProviders: () => <p>Other providers panel</p>,
}));

/** The default settings with `provider` writing the notes. */
function settingsWith(provider: Generated.LlmProvider): Generated.Settings {
  const settings = structuredClone(DEFAULT_SETTINGS);
  return { ...settings, ai: { ...settings.ai, provider } };
}

function renderSection() {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <AiProviderSettings />
    </QueryClientProvider>,
  );
}

describe('Settings → Notes → AI', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('shows the three choices, with other providers closed and no auto-summary switch', async () => {
    bindings.get.mockResolvedValue({ status: 'ok', data: settingsWith('claude') });
    renderSection();

    expect(await screen.findByText('Notes provider picker')).toBeTruthy();
    expect(screen.queryByText('Other providers panel')).toBeNull();
    expect(screen.queryByRole('switch')).toBeNull();
    fireEvent.click(screen.getByRole('button', { name: copy.otherProviders }));
    expect(screen.getByText('Other providers panel')).toBeTruthy();
  });

  it('says when another provider writes the notes', async () => {
    bindings.get.mockResolvedValue({ status: 'ok', data: settingsWith('ollama') });
    renderSection();

    expect(await screen.findByText('Notes use Ollama, set under Other providers.')).toBeTruthy();
  });
});
