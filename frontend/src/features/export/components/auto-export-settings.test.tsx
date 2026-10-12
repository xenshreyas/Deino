import { QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

import { DEFAULT_SETTINGS } from '@/lib/ipc/generated/settings-defaults';
import { createQueryClient } from '@/lib/ipc';

import { exportCopy } from '../copy';
import { AutoExportSettings } from './auto-export-settings';

vi.mock('@/lib/ipc/generated/settings', () => ({ commands: {}, events: {} }));
vi.mock('@/lib/ipc/generated/export', () => ({ commands: {}, events: {} }));

function renderSection() {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <AutoExportSettings settings={structuredClone(DEFAULT_SETTINGS.export)} />
    </QueryClientProvider>,
  );
}

describe('Settings → Connections → Auto-export', () => {
  it('shows the switch and the folder, the rest under More options (UX-15)', () => {
    renderSection();

    expect(screen.getAllByRole('switch')).toHaveLength(1);
    expect(screen.getByRole('switch', { name: exportCopy.autoToggle })).toBeTruthy();
    expect(screen.getByText(exportCopy.folderLabel)).toBeTruthy();
    expect(screen.queryByText(exportCopy.obsidianLabel)).toBeNull();

    fireEvent.click(screen.getByRole('button', { name: 'More options' }));

    expect(screen.getByRole('switch', { name: exportCopy.obsidianLabel })).toBeTruthy();
    expect(screen.getByText(exportCopy.fileNameLabel)).toBeTruthy();
  });
});
