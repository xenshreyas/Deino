import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

import type { DownloadView } from '../download-view';
import type { LocalModel } from '../hooks/use-local-models';
import { AutoStart, TranscriptionStatus } from './model-download-row';

function model(view: DownloadView, download = vi.fn()): LocalModel {
  return { view, sizeBytes: 680_000_000, download, error: null };
}

describe('the AI step’s transcription line (UX-23)', () => {
  it('reads as plain words with a percentage, and no model name', () => {
    render(<TranscriptionStatus model={model({ kind: 'downloading', percent: 42 })} />);

    expect(screen.getByText('Getting transcription ready… 42%')).toBeTruthy();
    expect(screen.getByRole('progressbar').getAttribute('aria-valuenow')).toBe('42');
    expect(document.body.textContent).not.toMatch(/Parakeet|MB/);
  });

  it('says when it is ready', () => {
    render(<TranscriptionStatus model={model({ kind: 'ready' })} />);

    expect(screen.getByText('Transcription is ready')).toBeTruthy();
    expect(screen.queryByRole('progressbar')).toBeNull();
  });

  it('starts a missing model once, without showing anything (the search model, UX-21)', () => {
    const download = vi.fn();
    const { container, rerender } = render(
      <AutoStart model={model({ kind: 'missing' }, download)} />,
    );
    rerender(<AutoStart model={model({ kind: 'missing' }, download)} />);

    expect(download).toHaveBeenCalledTimes(1);
    expect(container.textContent).toBe('');
  });
});
