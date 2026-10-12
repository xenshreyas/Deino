import { QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, test, vi } from 'vitest';

import type * as Privacy from '@/lib/ipc/generated/privacy';
import { createQueryClient } from '@/lib/ipc';

import { LibraryNotice } from './components/library-gate';
import { LockGate } from './components/lock-gate';
import { LockScreenView } from './components/lock-screen';
import { privacyCopy } from './copy';

const privacy = vi.hoisted(() => ({
  state: vi.fn<typeof Privacy.commands.state>(),
  touch: vi.fn<typeof Privacy.commands.touch>(),
  lock: vi.fn<typeof Privacy.commands.lock>(),
  unlock: vi.fn<typeof Privacy.commands.unlock>(),
  library: vi.fn<typeof Privacy.commands.library>(),
}));
vi.mock('@/lib/ipc/generated/privacy', () => ({ commands: privacy }));
vi.mock('@/features/recording', () => ({
  RecordingControls: () => <div>recording controls</div>,
  useRecordingState: () => ({ phase: 'idle' }),
}));

const copy = privacyCopy.lock;

function renderGate() {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <LockGate>
        <p>the library</p>
      </LockGate>
    </QueryClientProvider>,
  );
}

describe('the lock gate', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    privacy.touch.mockResolvedValue({ status: 'ok', data: 'locked' });
  });

  test('shows the app while unlocked', async () => {
    privacy.state.mockResolvedValue({ status: 'ok', data: 'unlocked' });
    renderGate();

    expect(await screen.findByText('the library')).toBeDefined();
  });

  test('hides the app while locked and unlocks through the prompt', async () => {
    privacy.state.mockResolvedValue({ status: 'ok', data: 'locked' });
    privacy.unlock.mockResolvedValue({ status: 'ok', data: 'unlocked' });
    renderGate();

    fireEvent.click(await screen.findByRole('button', { name: copy.unlock }));

    expect(await screen.findByText('the library')).toBeDefined();
    expect(privacy.unlock).toHaveBeenCalledTimes(1);
  });

  test('stays locked when the prompt is cancelled', async () => {
    privacy.state.mockResolvedValue({ status: 'ok', data: 'locked' });
    privacy.unlock.mockResolvedValue({
      status: 'error',
      error: { code: 'cancelled', message: 'unlocking was cancelled' },
    });
    renderGate();

    fireEvent.click(await screen.findByRole('button', { name: copy.unlock }));

    await waitFor(() => {
      expect(privacy.unlock).toHaveBeenCalled();
    });
    expect(screen.queryByText('the library')).toBeNull();
    expect(screen.getByText(copy.title)).toBeDefined();
  });

  test('pings activity on input', async () => {
    privacy.state.mockResolvedValue({ status: 'ok', data: 'unlocked' });
    renderGate();
    await screen.findByText('the library');

    fireEvent.keyDown(window, { key: 'a' });

    await waitFor(() => {
      expect(privacy.touch).toHaveBeenCalledTimes(1);
    });
  });
});

describe('the lock screen', () => {
  test('keeps the recording controls while recording', () => {
    render(<LockScreenView unlocking={false} recording onUnlock={vi.fn()} />);

    expect(screen.getByText('recording controls')).toBeDefined();
    expect(screen.getByText(copy.recording)).toBeDefined();
  });

  test('shows the waiting label while the prompt is open', () => {
    render(<LockScreenView unlocking recording={false} onUnlock={vi.fn()} />);

    const button = screen.getByRole('button', { name: copy.unlocking });
    expect(button.hasAttribute('disabled')).toBe(true);
  });
});

describe('the library notice', () => {
  test('shows progress while converting and nothing once ready', () => {
    const { container, rerender } = render(
      <LibraryNotice status={{ kind: 'converting', direction: 'encrypt' }} />,
    );
    expect(screen.getByText(privacyCopy.library.converting('encrypt'))).toBeDefined();

    rerender(
      <LibraryNotice status={{ kind: 'ready', encrypted: true, outcome: { kind: 'unchanged' } }} />,
    );
    expect(container.textContent).toBe('');
  });

  test('explains a missing key without offering to delete anything', () => {
    render(<LibraryNotice status={{ kind: 'unavailable', problem: 'key_missing' }} />);

    expect(screen.getByText(privacyCopy.library.problems.key_missing)).toBeDefined();
    expect(screen.queryByRole('button')).toBeNull();
  });
});
