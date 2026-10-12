import { QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import type { ReactNode } from 'react';
import { beforeEach, describe, expect, test, vi } from 'vitest';

import { defaultSettings } from '../settings/settings-fixture';
import type * as Retention from '@/lib/ipc/generated/retention';
import type * as Generated from '@/lib/ipc/generated/settings';
import { createQueryClient } from '@/lib/ipc';

import { PauseTipSlot } from './components/pause-tip';
import { RetentionSettings } from './components/retention-settings';
import { TranscriptGate } from './components/transcript-deleted';
import { retentionCopy } from './copy';
import type { MeetingId, RetentionSettings as Periods } from './types';

const settings = vi.hoisted(() => ({
  get: vi.fn<typeof Generated.commands.get>(),
  patch: vi.fn<typeof Generated.commands.patch>(),
}));
vi.mock('@/lib/ipc/generated/settings', () => ({ commands: settings }));

const retention = vi.hoisted(() => ({ status: vi.fn<typeof Retention.commands.status>() }));
vi.mock('@/lib/ipc/generated/retention', () => ({ commands: retention }));

const MEETING = 'meeting-1' as MeetingId;

function wrapper({ children }: { children: ReactNode }) {
  return <QueryClientProvider client={createQueryClient()}>{children}</QueryClientProvider>;
}

/** The settings with these retention periods and pause tip state. */
function withPrivacy(periods: Partial<Periods>, pauseTipSeen = false) {
  const base = defaultSettings();
  const privacy = { ...base.privacy, retention: { ...base.privacy.retention, ...periods } };
  return { ...base, privacy: { ...privacy, pauseTipSeen } };
}

beforeEach(() => {
  vi.clearAllMocks();
  settings.get.mockResolvedValue({ status: 'ok', data: defaultSettings() });
  settings.patch.mockImplementation(() =>
    Promise.resolve({ status: 'ok', data: withPrivacy({}, true) }),
  );
  retention.status.mockResolvedValue({
    status: 'ok',
    data: { audioDeletedAt: null, transcriptDeletedAt: null },
  });
});

describe('Settings → Retention', () => {
  test('shows the three periods and why transcripts follow recordings', async () => {
    settings.get.mockResolvedValue({
      status: 'ok',
      data: withPrivacy({ audio: 'days30', transcript: 'days90' }),
    });
    render(<RetentionSettings />, { wrapper });

    const copy = retentionCopy.settings;
    const audio = await screen.findByRole('combobox', { name: copy.audioTitle });
    expect(audio.textContent).toBe('30 days');
    expect(screen.getByRole('combobox', { name: copy.transcriptTitle }).textContent).toBe(
      '90 days',
    );
    expect(screen.getByRole('combobox', { name: copy.everythingTitle }).textContent).toBe('Never');
    expect(screen.getByText(copy.transcriptDescription)).toBeDefined();
  });
});

describe('TranscriptGate', () => {
  test('shows the transcript while it is kept', async () => {
    render(<TranscriptGate meetingId={MEETING}>transcript lines</TranscriptGate>, { wrapper });

    expect(await screen.findByText('transcript lines')).toBeDefined();
    expect(retention.status).toHaveBeenCalledWith(MEETING);
  });

  test('says "Transcript deleted" once retention deleted it', async () => {
    retention.status.mockResolvedValue({
      status: 'ok',
      data: { audioDeletedAt: null, transcriptDeletedAt: '2026-10-01T06:00:00Z' },
    });
    render(<TranscriptGate meetingId={MEETING}>transcript lines</TranscriptGate>, { wrapper });

    expect(await screen.findByText(retentionCopy.deleted.transcriptTitle)).toBeDefined();
    expect(screen.queryByText('transcript lines')).toBeNull();
  });
});

describe('PauseTipSlot', () => {
  test('shows the tip until it is dismissed, then saves that it was seen', async () => {
    render(<PauseTipSlot />, { wrapper });

    fireEvent.click(await screen.findByRole('button', { name: retentionCopy.tip.dismiss }));

    await waitFor(() => {
      expect(screen.queryByText(retentionCopy.tip.title)).toBeNull();
    });
    expect(settings.patch).toHaveBeenCalledWith({ privacy: { pauseTipSeen: true } });
  });

  test('a recording that ends with the tip up marks it seen', async () => {
    const view = render(<PauseTipSlot />, { wrapper });
    expect(await screen.findByText(retentionCopy.tip.title)).toBeDefined();

    view.unmount();

    expect(settings.patch).toHaveBeenCalledWith({ privacy: { pauseTipSeen: true } });
  });

  test('never shows again once seen', async () => {
    settings.get.mockResolvedValue({ status: 'ok', data: withPrivacy({}, true) });
    render(<PauseTipSlot />, { wrapper });

    await waitFor(() => {
      expect(settings.get).toHaveBeenCalled();
    });
    expect(screen.queryByText(retentionCopy.tip.title)).toBeNull();
  });
});
