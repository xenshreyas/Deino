/**
 * Regression guards for long meetings (TRN-01: smooth with 5,000 segments):
 * the list keeps only the rows near the viewport in the DOM, renders and
 * searches a two-hour meeting fast, and keyboard moves stay on the lines.
 * happy-dom has no layout, so the scroll area and rows get fixed sizes.
 */
import { QueryClientProvider } from '@tanstack/react-query';
import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { afterAll, beforeAll, describe, expect, it, vi } from 'vitest';

import {
  LONG_MEETING_ID,
  longMeetingParticipants,
  longTranscript,
  PHRASE,
} from '@/dev/mock-ipc/long-meeting';
import { createQueryClient } from '@/lib/ipc';
import { TooltipProvider } from '@/ui/tooltip';

import { mergeLines } from '../model/lines';
import { buildRows } from '../model/rows';
import { SpeakerDirectory } from '../model/speakers';
import { TranscriptView } from './transcript-view';

vi.mock('@/lib/ipc/generated/recording', () => ({ commands: {}, events: {} }));
// The "Transcript changed" banner reads the notes: none here.
vi.mock('@/lib/ipc/generated/enhance', () => ({
  commands: { get: () => Promise.resolve({ status: 'ok', data: null }) },
}));

const SEGMENTS = 5_000;
/** Generous for happy-dom on a busy CI machine; a real browser is far faster. */
const RENDER_BUDGET_MS = 1_500;
/** Rows a 600px view of 40px rows may hold, with overscan on both sides. */
const MAX_DOM_ROWS = 40;

const meeting = longTranscript(SEGMENTS);
const directory = new SpeakerDirectory(longMeetingParticipants());
const rows = buildRows(mergeLines(meeting.segments, new Map()), directory);

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

function renderView(live: boolean) {
  return render(
    <QueryClientProvider client={createQueryClient()}>
      <TooltipProvider>
        <TranscriptView
          meetingId={LONG_MEETING_ID}
          rows={rows}
          speakers={directory.all}
          live={live}
        />
      </TooltipProvider>
    </QueryClientProvider>,
  );
}

describe('TranscriptView with a two-hour meeting', () => {
  it(`renders ${String(SEGMENTS)} lines with only the visible rows in the DOM`, () => {
    const started = performance.now();
    renderView(false);
    const elapsed = performance.now() - started;

    const items = screen.getAllByRole('listitem');
    expect(items.length).toBeGreaterThan(0);
    expect(items.length).toBeLessThanOrEqual(MAX_DOM_ROWS);
    expect(elapsed).toBeLessThan(RENDER_BUDGET_MS);
  });

  it('opens a live transcript at its newest line', () => {
    renderView(true);

    const last = meeting.segments.at(-1)?.text ?? '';
    expect(screen.getByText(last)).toBeTruthy();
    expect(screen.getAllByRole('listitem').length).toBeLessThanOrEqual(MAX_DOM_ROWS);
  });

  it('searches every line and keeps the DOM small while jumping between hits', () => {
    renderView(false);
    const box = screen.getByRole('searchbox', { name: 'Search transcript' });

    const started = performance.now();
    fireEvent.change(box, { target: { value: PHRASE } });
    const elapsed = performance.now() - started;

    expect(within(screen.getByRole('search')).getByRole('status').textContent).toBe('1 of 20');
    fireEvent.keyDown(box, { key: 'Enter', shiftKey: true });
    expect(within(screen.getByRole('search')).getByRole('status').textContent).toBe('20 of 20');
    expect(screen.getAllByRole('listitem').length).toBeLessThanOrEqual(MAX_DOM_ROWS);
    expect(elapsed).toBeLessThan(RENDER_BUDGET_MS);
  });

  it('moves focus between lines with the arrow keys', async () => {
    renderView(false);
    const [first] = screen.getAllByRole('listitem');
    if (first === undefined) throw new Error('no rows rendered');

    act(() => {
      first.focus();
    });
    fireEvent.keyDown(first, { key: 'ArrowDown' });

    await waitFor(() => {
      expect(document.activeElement?.getAttribute('data-bubble')).toBe('1');
    });
  });
});
