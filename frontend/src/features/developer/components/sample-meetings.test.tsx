import { QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import type * as Generated from '@/lib/ipc/generated/dev';
import { createQueryClient } from '@/lib/ipc';

import { copy } from '../copy';
import { SampleMeetings } from './sample-meetings';

// Typed against the generated bindings: a Rust signature change breaks this mock.
const commands = vi.hoisted(
  () =>
    ({
      seedFixtures: vi.fn<typeof Generated.commands.seedFixtures>(),
    }) satisfies typeof Generated.commands,
);
vi.mock('@/lib/ipc/generated/dev', () => ({ commands }));

function renderSettings() {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <SampleMeetings />
    </QueryClientProvider>,
  );
}

describe('SampleMeetings', () => {
  beforeEach(() => {
    vi.resetAllMocks();
  });

  it('seeds the fixtures and reports how many meetings it loaded', async () => {
    commands.seedFixtures.mockResolvedValue({
      status: 'ok',
      data: { meetings: 8, segments: 1808 },
    });
    renderSettings();

    fireEvent.click(screen.getByRole('button', { name: copy.seed }));

    expect((await screen.findByRole('status')).textContent).toBe(copy.seeded(8, 1808));
    expect(commands.seedFixtures).toHaveBeenCalledOnce();
  });

  it('shows the error message when seeding fails', async () => {
    commands.seedFixtures.mockResolvedValue({
      status: 'error',
      error: { code: 'internal', message: 'could not write the fixtures' },
    });
    renderSettings();

    fireEvent.click(screen.getByRole('button', { name: copy.seed }));

    expect((await screen.findByRole('alert')).textContent).toBe('could not write the fixtures');
  });
});
