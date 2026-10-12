import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

import { screenCopy } from '../copy';
import type { ScreenSettings } from '../types';
import { CaptureCard } from './capture-card';
import { PermissionCard } from './permission-card';

const OFF: ScreenSettings = { detectedMeetings: false, video: true, videoFps: 1, snapshots: true };

describe('PermissionCard', () => {
  it('offers System Settings only while Screen Recording is missing', () => {
    const onOpenSettings = vi.fn();
    const { rerender } = render(
      <PermissionCard
        supported
        permitted={false}
        opening={false}
        openFailed={false}
        onOpenSettings={onOpenSettings}
      />,
    );

    expect(screen.getByText(screenCopy.permission.missing)).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: screenCopy.permission.openSettings }));
    expect(onOpenSettings).toHaveBeenCalledOnce();

    rerender(
      <PermissionCard
        supported
        permitted
        opening={false}
        openFailed={false}
        onOpenSettings={onOpenSettings}
      />,
    );
    expect(screen.getByText(screenCopy.permission.granted)).toBeTruthy();
    expect(screen.queryByRole('button')).toBeNull();
  });

  it('says when this computer cannot capture, and when opening failed', () => {
    render(
      <PermissionCard
        supported={false}
        permitted={false}
        opening={false}
        openFailed
        onOpenSettings={vi.fn()}
      />,
    );

    expect(screen.getByText(screenCopy.permission.unsupported)).toBeTruthy();
    expect(screen.getByRole('alert').textContent).toBe(screenCopy.permission.settingsFailed);
  });
});

describe('CaptureCard', () => {
  it('is off by default, with the other switches waiting for it', () => {
    const onChange = vi.fn();
    render(<CaptureCard screen={OFF} saving={false} onChange={onChange} />);

    const detected = screen.getByRole('switch', { name: screenCopy.capture.detected });
    expect(detected.getAttribute('aria-checked')).toBe('false');
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
    fireEvent.click(screen.getByRole('switch', { name: screenCopy.capture.video }));
    expect(onChange).toHaveBeenCalledWith({ video: false });
    fireEvent.click(screen.getByRole('radio', { name: screenCopy.capture.rates[5] }));
    expect(onChange).toHaveBeenCalledWith({ videoFps: 5 });
  });
});
