import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

import { recordingCopy } from '../copy';
import { EndingCardView } from './ending-card';

function renderCard() {
  const onKeep = vi.fn();
  const onStopNow = vi.fn();
  render(
    <EndingCardView
      ending={{ reason: 'micReleased', stopAt: new Date(Date.now() + 12_000).toISOString() }}
      seconds={12}
      busy={false}
      onKeep={onKeep}
      onStopNow={onStopNow}
    />,
  );
  return { onKeep, onStopNow };
}

describe('EndingCardView', () => {
  it('says when and why the recording ends, and keeps it going', async () => {
    const { onKeep } = renderCard();

    expect(screen.getByRole('heading', { name: 'Recording ends in 12s' })).toBeTruthy();
    expect(screen.getByText('The call ended')).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: recordingCopy.keepRecording }));
    // The card slides out first, then answers.
    await waitFor(() => {
      expect(onKeep).toHaveBeenCalledOnce();
    });
  });

  it('stops now from the caret menu', async () => {
    const { onKeep, onStopNow } = renderCard();

    fireEvent.pointerDown(screen.getByRole('button', { name: recordingCopy.endingMore }), {
      button: 0,
      ctrlKey: false,
    });
    fireEvent.click(await screen.findByRole('menuitem', { name: recordingCopy.stopNow }));
    await waitFor(() => {
      expect(onStopNow).toHaveBeenCalledOnce();
    });
    expect(onKeep).not.toHaveBeenCalled();
  });
});
