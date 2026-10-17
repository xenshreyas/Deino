import { QueryClientProvider } from '@tanstack/react-query';
import type { EventCallback } from '@tauri-apps/api/event';
import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { afterAll, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';

import type * as Speakers from '@/lib/ipc/generated/speakers';
import type * as Generated from '@/lib/ipc/generated/transcript';
import { createQueryClient } from '@/lib/ipc';
import { TooltipProvider } from '@/ui/tooltip';

import { liveUpdate, MEETING, segment, speaker } from '../test-data';
import type { SegmentUpdate, Transcript } from '../types';
import { TranscriptPanel, type TranscriptPanelProps } from './transcript-panel';

// Typed against the generated bindings: a Rust signature change breaks these mocks.
const bindings = vi.hoisted(() => {
  const handlers: { liveSegment?: EventCallback<unknown> } = {};
  const listen = vi.fn((handler: EventCallback<unknown>) => {
    handlers.liveSegment = handler;
    return Promise.resolve(vi.fn());
  });
  const idle = { listen: vi.fn(() => Promise.resolve(vi.fn())) };
  return {
    handlers,
    get: vi.fn<typeof Generated.commands.get>(),
    events: { liveSegment: { listen }, health: idle, recordingState: idle },
  };
});
vi.mock('@/lib/ipc/generated/transcript', () => ({ commands: { get: bindings.get } }));
vi.mock('@/lib/ipc/generated/recording', () => ({ commands: {}, events: bindings.events }));

const speakers = vi.hoisted(
  () =>
    ({
      participants: vi.fn<typeof Speakers.commands.participants>(),
      candidates: vi.fn<typeof Speakers.commands.candidates>(),
      rename: vi.fn<typeof Speakers.commands.rename>(),
      merge: vi.fn<typeof Speakers.commands.merge>(),
      confirmSuggestion: vi.fn<typeof Speakers.commands.confirmSuggestion>(),
      dismissSuggestion: vi.fn<typeof Speakers.commands.dismissSuggestion>(),
      voiceProfiles: vi.fn<typeof Speakers.commands.voiceProfiles>(),
      deleteVoiceprint: vi.fn<typeof Speakers.commands.deleteVoiceprint>(),
      forgetVoice: vi.fn<typeof Speakers.commands.forgetVoice>(),
    }) satisfies typeof Speakers.commands,
);
vi.mock('@/lib/ipc/generated/speakers', () => ({ commands: speakers }));

const PRIYA = speaker('priya', 'Priya Shah');
const ME = speaker('me', 'Shreyas', { isMe: true, color: 1 });
const PRIYA_PERSON = { id: 'person-priya' as Speakers.PersonId, name: 'Priya Shah' };

const TRANSCRIPT: Transcript = {
  segments: [
    segment('a', 1, 'The budget looks fine to me.', 'priya'),
    segment('b', 3, 'Then the budget is approved.', 'me'),
    segment('c', 6, 'Great, ship it Monday.', 'priya'),
  ],
};

/** Gives happy-dom a layout: a 600px scroll area of 40px rows. */
const offsetHeight = Object.getOwnPropertyDescriptor(HTMLElement.prototype, 'offsetHeight');
beforeAll(() => {
  Object.defineProperty(HTMLElement.prototype, 'offsetHeight', {
    configurable: true,
    get(this: HTMLElement) {
      return this.getAttribute('role') === 'region' ? 600 : 40;
    },
  });
});
afterAll(() => {
  if (offsetHeight) Object.defineProperty(HTMLElement.prototype, 'offsetHeight', offsetHeight);
});

beforeEach(() => {
  vi.clearAllMocks();
  bindings.get.mockResolvedValue({ status: 'ok', data: TRANSCRIPT });
  speakers.participants.mockResolvedValue({ status: 'ok', data: [ME, PRIYA] });
  speakers.candidates.mockResolvedValue({
    status: 'ok',
    data: [{ person: PRIYA_PERSON, isAttendee: true, isMe: false }],
  });
  speakers.rename.mockResolvedValue({
    status: 'ok',
    data: { person: PRIYA_PERSON, speakerId: PRIYA.speakerId, enrolled: false },
  });
});

function renderPanel(props: Partial<TranscriptPanelProps> = {}) {
  return render(
    <QueryClientProvider client={createQueryClient()}>
      <TooltipProvider>
        <TranscriptPanel meetingId={MEETING} live={false} {...props} />
      </TooltipProvider>
    </QueryClientProvider>,
  );
}

function emit(update: SegmentUpdate) {
  act(() => {
    bindings.handlers.liveSegment?.({ event: 'live-segment', id: 0, payload: { update } });
  });
}

const searchBox = () => screen.getByRole('searchbox', { name: 'Search transcript' });
const counter = () => within(screen.getByRole('search')).getByRole('status');

describe('TranscriptPanel', () => {
  it('shows each line under its speaker turn, named from the participants', async () => {
    renderPanel();

    expect(await screen.findByText('The budget looks fine to me.')).toBeTruthy();
    expect(screen.getAllByRole('listitem')).toHaveLength(3);
    expect(screen.getAllByText('Priya Shah')).toHaveLength(2);
    expect(screen.getByText('Shreyas')).toBeTruthy();
  });

  it('moves through hits with Enter and Shift+Enter and counts them', async () => {
    renderPanel();
    await screen.findByText('Great, ship it Monday.');

    fireEvent.change(searchBox(), { target: { value: 'budget' } });
    expect(counter().textContent).toBe('1 of 2');
    fireEvent.keyDown(searchBox(), { key: 'Enter' });
    expect(counter().textContent).toBe('2 of 2');
    fireEvent.keyDown(searchBox(), { key: 'Enter', shiftKey: true });
    expect(counter().textContent).toBe('1 of 2');
    expect(document.querySelector('mark[aria-current="true"]')?.textContent).toBe('budget');

    fireEvent.keyDown(searchBox(), { key: 'Escape' });
    expect((searchBox() as HTMLInputElement).value).toBe('');
  });

  it('turns speaker: into a chip that filters the hits, and Backspace removes it', async () => {
    renderPanel();
    await screen.findByText('Great, ship it Monday.');

    fireEvent.change(searchBox(), { target: { value: 'speaker:priya ' } });

    expect(screen.getByRole('button', { name: 'Remove Priya Shah filter' })).toBeTruthy();
    expect(counter().textContent).toBe('1 of 2');
    fireEvent.change(searchBox(), { target: { value: 'budget' } });
    expect(counter().textContent).toBe('1 of 1');
    fireEvent.change(searchBox(), { target: { value: '' } });
    fireEvent.keyDown(searchBox(), { key: 'Backspace' });
    expect(screen.queryByRole('button', { name: 'Remove Priya Shah filter' })).toBeNull();
  });

  it('focuses the search field on ⌘F', async () => {
    renderPanel();
    await screen.findByText('Great, ship it Monday.');

    fireEvent.keyDown(window, { key: 'f', metaKey: true });

    expect(document.activeElement).toBe(searchBox());
  });

  it('grows a live line in place and settles it when it is saved', async () => {
    renderPanel({ live: true });
    await screen.findByText('Great, ship it Monday.');

    emit(liveUpdate('d', 'open', 'One more'));
    const open = await screen.findByText('One more');
    expect(open.closest('[role="listitem"]')?.getAttribute('aria-busy')).toBe('true');
    emit(liveUpdate('d', 'final', 'One more thing.'));

    const saved = await screen.findByText('One more thing.');
    expect(saved.closest('[role="listitem"]')?.hasAttribute('aria-busy')).toBe(false);
    expect(screen.getAllByRole('listitem')).toHaveLength(4);
  });

  it('marks a linked moment, reports it shown, and keeps it the linked location', async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    const onMomentShown = vi.fn();
    renderPanel({ moment: { kind: 'time', at: 3_500 }, onMomentShown });

    const moment = await screen.findByText('Then the budget is approved.');
    // The mark lands in an effect a commit after the text; wait for it.
    await waitFor(() => {
      expect(moment.closest('[role="listitem"]')?.getAttribute('aria-current')).toBe('location');
    });
    act(() => {
      vi.advanceTimersByTime(2_000);
    });
    expect(onMomentShown).toHaveBeenCalledOnce();
    // The highlight fades; the line stays the linked location.
    expect(moment.closest('[role="listitem"]')?.getAttribute('aria-current')).toBe('location');
    vi.useRealTimers();
  });

  it('marks a cited line (NOT-03 "Show in transcript"), and lets go of a missing one', async () => {
    const onMomentShown = vi.fn();
    const { unmount } = renderPanel({ moment: { kind: 'line', segmentId: 'c' }, onMomentShown });

    const cited = await screen.findByText('Great, ship it Monday.');
    expect(cited.closest('[role="listitem"]')?.getAttribute('aria-current')).toBe('location');
    unmount();

    renderPanel({ moment: { kind: 'line', segmentId: 'gone' }, onMomentShown });
    await screen.findByText('Great, ship it Monday.');
    expect(onMomentShown).toHaveBeenCalledOnce();
    expect(document.querySelector('[aria-current="location"]')).toBeNull();
  });

  it('copies a line from its toolbar', async () => {
    const writeText = vi.fn(() => Promise.resolve());
    Object.defineProperty(navigator, 'clipboard', { configurable: true, value: { writeText } });
    renderPanel();
    await screen.findByText('Great, ship it Monday.');

    fireEvent.click(screen.getAllByRole('button', { name: 'Copy' })[2] as HTMLElement);

    await waitFor(() => {
      expect(writeText).toHaveBeenCalledWith('Great, ship it Monday.');
    });
  });

  it('renames a speaker everywhere from its turn header with CAP-04\'s "Who is …?"', async () => {
    renderPanel();
    await screen.findByText('Great, ship it Monday.');

    fireEvent.click(screen.getByRole('button', { name: /Shreyas/ }));
    expect(await screen.findByText('Who is Shreyas?')).toBeTruthy();
    expect(screen.getByRole('radio', { name: 'All segments' }).getAttribute('aria-checked')).toBe(
      'true',
    );
    fireEvent.click(await screen.findByText('Priya Shah', { selector: '[role="option"] *' }));

    await waitFor(() => {
      expect(speakers.rename).toHaveBeenCalledWith(
        expect.objectContaining({
          speakerId: ME.speakerId,
          scope: { kind: 'all' },
        }),
      );
    });
  });

  it('rests with "No transcript yet", or "Listening" while recording', async () => {
    bindings.get.mockResolvedValue({ status: 'ok', data: { segments: [] } });
    const { unmount } = renderPanel();
    expect(await screen.findByText('No transcript yet')).toBeTruthy();
    unmount();

    renderPanel({ live: true });
    expect(await screen.findByText('Listening')).toBeTruthy();
  });

  it('explains a failed load inline and retries', async () => {
    bindings.get.mockResolvedValue({
      status: 'error',
      error: { code: 'internal', message: 'could not load the transcript' },
    });
    renderPanel();

    const retry = await screen.findByRole('button', { name: 'Try again' }, { timeout: 5_000 });
    bindings.get.mockResolvedValue({ status: 'ok', data: TRANSCRIPT });
    fireEvent.click(retry);

    expect(await screen.findByText('Great, ship it Monday.')).toBeTruthy();
  });
});
