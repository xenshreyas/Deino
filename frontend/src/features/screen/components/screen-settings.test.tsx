import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

import { screenCopy } from '../copy';
import type { ScreenSettings } from '../types';
import { CaptureCard } from './capture-card';

const OFF: ScreenSettings = {
  detectedMeetings: false,
  video: true,
  videoFps: 1,
  snapshots: true,
  describeSnapshots: false,
  consentRemindersShown: 0,
};

describe('CaptureCard', () => {
  it('shows one switch, off by default, with the rest under More options', () => {
    const onChange = vi.fn();
    render(<CaptureCard screen={OFF} saving={false} onChange={onChange} />);

    const detected = screen.getByRole('switch', { name: screenCopy.capture.detected });
    expect(detected.getAttribute('aria-checked')).toBe('false');
    expect(screen.getAllByRole('switch')).toHaveLength(1);
    fireEvent.click(screen.getByRole('button', { name: 'More options' }));
    expect(
      screen.getByRole('switch', { name: screenCopy.capture.snapshots }).hasAttribute('disabled'),
    ).toBe(true);
    expect(screen.queryByText(screenCopy.capture.consent)).toBeNull();

    fireEvent.click(detected);
    expect(onChange).toHaveBeenCalledWith({ detectedMeetings: true });
  });

  it('shows the consent reminder and the frame rate once on', () => {
    const onChange = vi.fn();
    render(
      <CaptureCard
        screen={{ ...OFF, detectedMeetings: true }}
        saving={false}
        onChange={onChange}
      />,
    );

    expect(screen.getByText(screenCopy.capture.consent)).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: 'More options' }));
    fireEvent.click(screen.getByRole('switch', { name: screenCopy.capture.video }));
    expect(onChange).toHaveBeenCalledWith({ video: false });
    fireEvent.click(screen.getByRole('radio', { name: screenCopy.capture.rates[5] }));
    expect(onChange).toHaveBeenCalledWith({ videoFps: 5 });
  });
});
