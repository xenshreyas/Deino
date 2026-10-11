import { QueryClientProvider } from '@tanstack/react-query';
import { act, render, type RenderResult } from '@testing-library/react';
import { useEffect, useState, type ReactNode } from 'react';
import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';

import { createQueryClient } from '@/lib/ipc';
import { SidebarProvider } from '@/components/Sidebar/SidebarProvider';
import { useMeetingData } from '@/hooks/meeting-details/useMeetingData';
import { useSummaryGeneration } from '@/hooks/meeting-details/useSummaryGeneration';
import type { MeetingSummary, SummaryProcessResponse, Transcript } from '@/types';

import { LegacyMeetingSession } from './meeting-session';

/** The sidebar reads meetings through TanStack Query, so it needs a client. */
function WithSidebar({ children }: { children: ReactNode }) {
  return (
    <QueryClientProvider client={useState(createQueryClient)[0]}>
      <SidebarProvider>{children}</SidebarProvider>
    </QueryClientProvider>
  );
}

const { invoke, notify } = vi.hoisted(() => ({ invoke: vi.fn(), notify: vi.fn() }));
vi.mock('@tauri-apps/api/core', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@tauri-apps/api/core')>()),
  invoke,
}));
vi.mock('sonner', () => ({
  toast: { info: notify, error: notify, success: notify, warning: notify },
}));
vi.mock('@/contexts/ConfigContext', () => ({ useConfig: () => ({ isAutoSummary: false }) }));
vi.mock('@tanstack/react-router', () => ({
  useLocation: (options: { select: (location: { pathname: string }) => string }) =>
    options.select({ pathname: '/meetings/meeting-a' }),
  useNavigate: () => vi.fn(),
}));

/** The props the screen passes to its content view (the real one renders BlockNote). */
interface ContentProps {
  meeting: { id: string; title: string; transcripts: Transcript[] };
  initialSummary: SummaryProcessResponse | null;
  summaryData: MeetingSummary | null;
  onRefetchTranscripts: () => Promise<void>;
}

// A lightweight content view around the real route, transcript loader, state hooks and polling provider.
let summaryState: ReturnType<typeof useSummaryGeneration>;
let refresh: () => Promise<void>;
let mounts = 0;
function SummaryView(props: ContentProps) {
  const data = useMeetingData(props);
  summaryState = useSummaryGeneration({
    meeting: props.meeting, initialSummary: props.initialSummary, transcripts: data.transcripts,
    modelConfig: { provider: 'ollama', model: 'test' }, isModelConfigLoading: false,
    selectedTemplate: 'daily_standup', setAiSummary: data.setAiSummary, updateMeetingTitle: data.updateMeetingTitle,
  });
  refresh = props.onRefetchTranscripts;
  useEffect(() => { mounts += 1; }, []);
  return <output>{props.meeting.id}:{summaryState.summaryStatus}:{JSON.stringify(data.aiSummary)}</output>;
}
vi.mock('./meeting-panels', () => ({ default: SummaryView }));

let selectedMeeting = 'meeting-a';
const metadata = (id: string) => ({ id, title: id, created_at: '2026-09-01', updated_at: '2026-09-01' });
let readMetadata: (id: string) => Promise<unknown>;
let readTranscripts: (id: string) => Promise<unknown>;
const transcriptPage = {
  transcripts: [{ id: 'transcript', text: 'Meeting transcript', timestamp: '00:00' }], total_count: 1, has_more: false,
};
let savedSummary: SummaryProcessResponse;
invoke.mockImplementation(async (command: string, args?: Record<string, unknown>) => {
  const meetingId = String(args?.meetingId);
  if (command === 'plugin:meetings|list') return [];
  if (command === 'api_get_summary') return { ...savedSummary, meeting_id: meetingId };
  if (command === 'api_get_meeting_metadata') return readMetadata(meetingId);
  if (command === 'api_get_meeting_transcripts') return readTranscripts(meetingId);
  if (command === 'api_process_transcript') return { process_id: 'attempt-b' };
  if (command === 'api_cancel_summary') return { cancelled: true };
  throw new Error(`Unexpected command: ${command}`);
});

let view: RenderResult | undefined;
const timers = new Map<number, () => Promise<void>>();
let nextTimer = 0;
const realSetInterval = globalThis.setInterval;
const realClearInterval = globalThis.clearInterval;
function fakeSetInterval(callback: () => Promise<void>): number {
  const id = ++nextTimer;
  timers.set(id, callback);
  return id;
}
function fakeClearInterval(id: number): void {
  timers.delete(id);
}
beforeEach(() => {
  view = undefined;
  selectedMeeting = 'meeting-a';
  readMetadata = async id => metadata(id);
  readTranscripts = async () => transcriptPage;
  savedSummary = { meeting_id: selectedMeeting, status: 'pending', start: 'attempt-a', end: null, data: null, error: null, meetingName: 'Meeting A' };
  mounts = 0; timers.clear(); invoke.mockClear(); notify.mockClear();
  globalThis.setInterval = fakeSetInterval as unknown as typeof setInterval;
  globalThis.clearInterval = fakeClearInterval as unknown as typeof clearInterval;
});
afterEach(async () => {
  await act(async () => view?.unmount());
  globalThis.setInterval = realSetInterval;
  globalThis.clearInterval = realClearInterval;
});
async function show() {
  const ui = <WithSidebar><LegacyMeetingSession meetingId={selectedMeeting}>{null}</LegacyMeetingSession></WithSidebar>;
  await act(async () => {
    if (view) view.rerender(ui); else view = render(ui);
  });
}
async function tick() {
  await act(async () => { for (const callback of [...timers.values()]) await callback(); });
}
const text = () => view?.container.textContent ?? '';
async function complete(markdown: string, attempt: string) {
  savedSummary = { ...savedSummary, status: 'completed', start: attempt, data: { markdown } };
  await tick();
}
function deferMetadata() {
  let resolve!: (value: unknown) => void;
  readMetadata = () => new Promise(done => { resolve = done; });
  return async () => { await act(async () => { resolve(metadata(selectedMeeting)); }); };
}

describe('meeting route transcript refresh', () => {
  test('navigation waits for the new meeting first transcript page before mounting its view', async () => {
    await show();
    await complete('Summary A', 'attempt-a');
    selectedMeeting = 'meeting-b';
    savedSummary = { ...savedSummary, meeting_id: selectedMeeting, status: 'idle', data: null, start: null };
    let resolvePage!: (value: unknown) => void;
    readTranscripts = () => new Promise(resolve => { resolvePage = resolve; });
    await show();
    expect(view?.container.querySelectorAll('output')).toHaveLength(0);
    expect(mounts).toBe(1);
    await act(async () => { resolvePage(transcriptPage); });
    expect(text()).toContain('meeting-b');
    expect(summaryState.summaryStatus).toBe('idle');
    expect(text()).not.toContain('Summary A');
    expect(mounts).toBe(2);
  });

  test('completed regeneration survives refresh without reviving the original pending attempt', async () => {
    await show();
    await complete('Summary A', 'attempt-a');
    await act(async () => { await summaryState.handleRegenerateSummary(); });
    await complete('Summary B', 'attempt-b');
    expect(text()).toContain('Summary B');
    const finishRefresh = deferMetadata();
    await act(async () => { void refresh(); });
    expect(text()).toContain('Summary B');
    expect(timers.size).toBe(0);
    expect(mounts).toBe(1);
    await finishRefresh();
    await tick();
    expect(summaryState.summaryStatus).toBe('completed');
    expect(text()).toContain('Summary B');
    expect(timers.size).toBe(0);
    expect(mounts).toBe(1);
  });

  test('refresh during regeneration keeps the current attempt and its Stop token', async () => {
    await show();
    await complete('Summary A', 'attempt-a');
    await act(async () => { await summaryState.handleRegenerateSummary(); });
    const currentTimer = [...timers.keys()][0];
    const finishRefresh = deferMetadata();
    await act(async () => { void refresh(); });
    expect([...timers.keys()]).toEqual([currentTimer]);
    expect(summaryState.summaryStatus).toBe('regenerating');
    await finishRefresh();
    expect([...timers.keys()]).toEqual([currentTimer]);
    expect(summaryState.summaryStatus).toBe('regenerating');
    await act(async () => { await summaryState.handleStopGeneration(); });
    const cancel = invoke.mock.calls.find(([command]) => command === 'api_cancel_summary');
    expect(cancel?.[1]).toMatchObject({ processId: 'attempt-b' });
    expect(timers.size).toBe(0);
  });
});
