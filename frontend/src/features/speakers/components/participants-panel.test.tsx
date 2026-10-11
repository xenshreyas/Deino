import { QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, test, vi } from 'vitest';

import type * as Pipeline from '@/lib/ipc/generated/pipeline';
import type * as Generated from '@/lib/ipc/generated/speakers';
import { createQueryClient } from '@/lib/ipc';
import { TooltipProvider } from '@/ui/tooltip';

import type { MeetingId, Participant, PersonId, SpeakerId } from '../types';
import { ParticipantsPanel } from './participants-panel';

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

const pipeline = vi.hoisted(
  () =>
    ({
      status: vi.fn<typeof Pipeline.commands.status>(),
      rerun: vi.fn<typeof Pipeline.commands.rerun>(),
    }) satisfies typeof Pipeline.commands,
);
vi.mock('@/lib/ipc/generated/pipeline', () => ({ commands: pipeline, events: {} }));

const MEETING = 'meeting-1' as MeetingId;
const MAYA = { id: 'person-maya' as PersonId, name: 'Maya Chen' };
const PRIYA = { id: 'person-priya' as PersonId, name: 'Priya Nair' };

function row(id: string, change: Partial<Participant>): Participant {
  return {
    speakerId: id as SpeakerId,
    label: 'Speaker 2',
    color: 2,
    isMe: false,
    person: null,
    speech: 60_000,
    sharePercent: 50,
    suggestion: null,
    ...change,
  };
}

const named = row('speaker-maya', { person: MAYA, speech: 724_000, sharePercent: 75 });
const unsure = row('speaker-3', {
  label: 'Speaker 3',
  color: 3,
  sharePercent: 25,
  suggestion: { person: PRIYA, score: 0.64 },
});

function renderPanel() {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <TooltipProvider>
        <ParticipantsPanel meetingId={MEETING} />
      </TooltipProvider>
    </QueryClientProvider>,
  );
}

describe('the participants panel', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    commands.participants.mockResolvedValue({ status: 'ok', data: [named, unsure] });
    commands.candidates.mockResolvedValue({ status: 'ok', data: [] });
    commands.confirmSuggestion.mockResolvedValue({
      status: 'ok',
      data: { person: PRIYA, speakerId: unsure.speakerId, enrolled: true },
    });
    commands.dismissSuggestion.mockResolvedValue({ status: 'ok', data: null });
    pipeline.rerun.mockResolvedValue({ status: 'ok', data: [] });
  });

  test('lists each speaker with their name, talk time and share', async () => {
    renderPanel();

    expect(await screen.findByText('Maya Chen')).toBeDefined();
    expect(screen.getByLabelText('Maya Chen spoke for 12m 04s, 75% of the meeting')).toBeDefined();
    expect(screen.getByText('Speaker 3')).toBeDefined();
  });

  test('"Yes" names the speaker and then offers to regenerate the notes', async () => {
    renderPanel();

    fireEvent.click(await screen.findByRole('button', { name: 'Yes' }));

    await waitFor(() => {
      expect(commands.confirmSuggestion).toHaveBeenCalledWith(MEETING, unsure.speakerId);
    });
    fireEvent.click(await screen.findByRole('button', { name: 'Regenerate notes' }));
    await waitFor(() => {
      expect(pipeline.rerun).toHaveBeenCalledWith(MEETING, 'legacy_summary');
    });
  });

  test('"No" forgets the suggestion without touching the notes', async () => {
    renderPanel();

    fireEvent.click(await screen.findByRole('button', { name: 'No' }));

    await waitFor(() => {
      expect(commands.dismissSuggestion).toHaveBeenCalledWith(unsure.speakerId);
    });
    expect(screen.queryByRole('button', { name: 'Regenerate notes' })).toBeNull();
  });

  test('a meeting without speakers explains when they appear', async () => {
    commands.participants.mockResolvedValue({ status: 'ok', data: [] });
    renderPanel();

    expect(await screen.findByText('No speakers yet')).toBeDefined();
  });
});
