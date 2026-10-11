import { QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, test, vi } from 'vitest';

import type * as Generated from '@/lib/ipc/generated/speakers';
import { createQueryClient } from '@/lib/ipc';
import { TooltipProvider } from '@/ui/tooltip';

import type { MeetingId, PersonId, VoiceprintId, VoiceProfile } from '../types';
import { VoiceProfiles } from './voice-profiles';

const commands = vi.hoisted(
  () =>
    ({
      participants: vi.fn<typeof Generated.commands.participants>(),
      candidates: vi.fn<typeof Generated.commands.candidates>(),
      rename: vi.fn<typeof Generated.commands.rename>(),
      merge: vi.fn<typeof Generated.commands.merge>(),
      confirmSuggestion: vi.fn<typeof Generated.commands.confirmSuggestion>(),
      dismissSuggestion: vi.fn<typeof Generated.commands.dismissSuggestion>(),
      voiceProfiles: vi.fn<typeof Generated.commands.voiceProfiles>(),
      deleteVoiceprint: vi.fn<typeof Generated.commands.deleteVoiceprint>(),
      forgetVoice: vi.fn<typeof Generated.commands.forgetVoice>(),
    }) satisfies typeof Generated.commands,
);
vi.mock('@/lib/ipc/generated/speakers', () => ({ commands }));

const SAMPLE = 'voiceprint-1' as VoiceprintId;
const maya: VoiceProfile = {
  personId: 'person-maya' as PersonId,
  name: 'Maya Chen',
  isMe: false,
  voiceprints: [
    {
      id: SAMPLE,
      sourceMeetingId: 'meeting-1' as MeetingId,
      sourceMeetingTitle: 'Weekly standup',
      quality: 0.82,
      createdAt: '2026-10-08T09:00:00Z',
    },
    {
      id: 'voiceprint-2' as VoiceprintId,
      sourceMeetingId: null,
      sourceMeetingTitle: null,
      quality: 0.4,
      createdAt: '2026-10-01T09:00:00Z',
    },
  ],
};

function renderProfiles() {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <TooltipProvider>
        <VoiceProfiles />
      </TooltipProvider>
    </QueryClientProvider>,
  );
}

describe('voice profiles', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    commands.voiceProfiles.mockResolvedValue({ status: 'ok', data: [maya] });
    commands.deleteVoiceprint.mockResolvedValue({ status: 'ok', data: null });
    commands.forgetVoice.mockResolvedValue({ status: 'ok', data: null });
  });

  test('each person shows their samples, where they came from and how clear they are', async () => {
    renderProfiles();

    fireEvent.click(await screen.findByRole('button', { name: /Maya Chen/u }));

    expect(await screen.findByText('From Weekly standup')).toBeDefined();
    expect(screen.getByText('From a deleted meeting')).toBeDefined();
    expect(screen.getByText(/82% clear/u)).toBeDefined();
  });

  test('a sample can be deleted, and a whole voice forgotten after confirming', async () => {
    renderProfiles();
    fireEvent.click(await screen.findByRole('button', { name: /Maya Chen/u }));

    fireEvent.click(
      (await screen.findAllByRole('button', { name: 'Delete sample' }))[0] as HTMLElement,
    );
    await waitFor(() => {
      expect(commands.deleteVoiceprint).toHaveBeenCalledWith(SAMPLE);
    });
    fireEvent.click(screen.getByRole('button', { name: 'Forget Maya Chen’s voice' }));
    fireEvent.click(await screen.findByRole('button', { name: 'Forget voice' }));
    await waitFor(() => {
      expect(commands.forgetVoice).toHaveBeenCalledWith(maya.personId);
    });
  });

  test('without profiles it says how to make one', async () => {
    commands.voiceProfiles.mockResolvedValue({ status: 'ok', data: [] });
    renderProfiles();

    expect(await screen.findByText('No voice profiles yet')).toBeDefined();
  });
});
