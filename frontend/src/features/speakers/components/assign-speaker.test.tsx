import { QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, test, vi } from 'vitest';

import type * as Generated from '@/lib/ipc/generated/speakers';
import { createQueryClient } from '@/lib/ipc';

import type { MeetingId, NameCandidate, PersonId, SegmentId, SpeakerId } from '../types';
import { AssignSpeaker } from './assign-speaker';

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

const MEETING = 'meeting-1' as MeetingId;
const SPEAKER = 'speaker-2' as SpeakerId;
const SEGMENT = 'segment-7' as SegmentId;
const maya: NameCandidate = {
  person: { id: 'person-maya' as PersonId, name: 'Maya Chen' },
  isAttendee: true,
  isMe: false,
};
const bob: NameCandidate = {
  person: { id: 'person-bob' as PersonId, name: 'Bob Stone' },
  isAttendee: false,
  isMe: false,
};

function renderAssign(segmentId?: SegmentId) {
  const onRenamed = vi.fn();
  const onDone = vi.fn();
  render(
    <QueryClientProvider client={createQueryClient()}>
      <AssignSpeaker
        meetingId={MEETING}
        speakerId={SPEAKER}
        name="Speaker 2"
        segmentId={segmentId}
        onRenamed={onRenamed}
        onDone={onDone}
      />
    </QueryClientProvider>,
  );
  return { onRenamed, onDone };
}

describe('"Who is Speaker 2?"', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    commands.candidates.mockResolvedValue({ status: 'ok', data: [maya, bob] });
    commands.rename.mockResolvedValue({
      status: 'ok',
      data: { person: maya.person, speakerId: SPEAKER, enrolled: true },
    });
  });

  test('groups attendees before people, and naming from the panel covers every segment', async () => {
    const { onRenamed, onDone } = renderAssign();

    expect(await screen.findByText('In this meeting')).toBeDefined();
    expect(screen.queryByRole('group', { name: 'Rename' })).toBeNull();
    fireEvent.click(screen.getByText('Maya Chen'));

    await waitFor(() => {
      expect(commands.rename).toHaveBeenCalledWith({
        meetingId: MEETING,
        speakerId: SPEAKER,
        name: { kind: 'person', personId: maya.person.id },
        scope: { kind: 'all' },
      });
    });
    expect(onDone).toHaveBeenCalled();
    await waitFor(() => {
      expect(onRenamed).toHaveBeenCalledWith(expect.anything(), 'all');
    });
  });

  test('from a segment it renames that segment unless "All segments" is chosen', async () => {
    renderAssign(SEGMENT);

    fireEvent.click(await screen.findByText('Bob Stone'));
    await waitFor(() => {
      expect(commands.rename).toHaveBeenLastCalledWith(
        expect.objectContaining({ scope: { kind: 'segment', segmentId: SEGMENT } }),
      );
    });
  });

  test('a typed name that matches nobody adds a new person', async () => {
    commands.candidates.mockResolvedValue({ status: 'ok', data: [] });
    renderAssign(SEGMENT);

    fireEvent.click(await screen.findByRole('radio', { name: 'All segments' }));
    fireEvent.change(screen.getByPlaceholderText('Search or type a name'), {
      target: { value: 'Dana Fox' },
    });
    fireEvent.click(await screen.findByText('Add “Dana Fox”'));

    await waitFor(() => {
      expect(commands.rename).toHaveBeenCalledWith(
        expect.objectContaining({
          name: { kind: 'new', name: 'Dana Fox' },
          scope: { kind: 'all' },
        }),
      );
    });
  });
});
