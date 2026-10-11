import { QueryClientProvider } from '@tanstack/react-query';
import { act, render, type RenderResult } from '@testing-library/react';
import { useEffect, useState, type ReactNode } from 'react';
import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';

import { createQueryClient } from '@/lib/ipc';
import { SidebarProvider } from '@/components/Sidebar/SidebarProvider';
import type { Transcript } from '@/types';

import { LegacyMeetingSession } from './meeting-session';

/** The sidebar reads meetings through TanStack Query, so it needs a client. */
function WithSidebar({ children }: { children: ReactNode }) {
  return (
    <QueryClientProvider client={useState(createQueryClient)[0]}>
      <SidebarProvider>{children}</SidebarProvider>
    </QueryClientProvider>
  );
}

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@tauri-apps/api/core')>()),
  invoke,
}));
vi.mock('@tanstack/react-router', () => ({
  useLocation: (options: { select: (location: { pathname: string }) => string }) =>
    options.select({ pathname: '/meetings/meeting-a' }),
  useNavigate: () => vi.fn(),
}));

/** The props the session passes to its transcript view. */
interface ContentProps {
  meeting: { id: string; title: string; transcripts: Transcript[] };
}

let mounts = 0;
function TranscriptView(props: ContentProps) {
  useEffect(() => {
    mounts += 1;
  }, []);
  return (
    <output>
      {props.meeting.id}:{props.meeting.transcripts.length}
    </output>
  );
}
vi.mock('./meeting-panels', () => ({ default: TranscriptView }));

let selectedMeeting = 'meeting-a';
const metadata = (id: string) => ({
  id,
  title: id,
  created_at: '2026-09-01',
  updated_at: '2026-09-01',
});
let readTranscripts: (id: string) => Promise<unknown>;
const transcriptPage = {
  transcripts: [{ id: 'transcript', text: 'Meeting transcript', timestamp: '00:00' }],
  total_count: 1,
  has_more: false,
};
invoke.mockImplementation(async (command: string, args?: Record<string, unknown>) => {
  const meetingId = String(args?.meetingId);
  if (command === 'plugin:meetings|list') return [];
  if (command === 'api_get_meeting_metadata') return metadata(meetingId);
  if (command === 'api_get_meeting_transcripts') return readTranscripts(meetingId);
  throw new Error(`Unexpected command: ${command}`);
});

let view: RenderResult | undefined;
beforeEach(() => {
  view = undefined;
  selectedMeeting = 'meeting-a';
  readTranscripts = async () => transcriptPage;
  mounts = 0;
  invoke.mockClear();
});
afterEach(async () => {
  await act(async () => view?.unmount());
});
async function show() {
  const ui = (
    <WithSidebar>
      <LegacyMeetingSession meetingId={selectedMeeting}>{null}</LegacyMeetingSession>
    </WithSidebar>
  );
  await act(async () => {
    if (view) view.rerender(ui);
    else view = render(ui);
  });
}
const text = () => view?.container.textContent ?? '';

describe('meeting route transcript refresh', () => {
  test('navigation waits for the new meeting first transcript page before mounting its view', async () => {
    await show();
    expect(text()).toContain('meeting-a:1');
    selectedMeeting = 'meeting-b';
    let resolvePage!: (value: unknown) => void;
    readTranscripts = () =>
      new Promise((resolve) => {
        resolvePage = resolve;
      });
    await show();
    expect(view?.container.querySelectorAll('output')).toHaveLength(0);
    expect(mounts).toBe(1);
    await act(async () => {
      resolvePage(transcriptPage);
    });
    expect(text()).toContain('meeting-b:1');
    expect(mounts).toBe(2);
  });

  test('the session never asks for the legacy summary', async () => {
    await show();
    const commands = invoke.mock.calls.map(([command]) => String(command));
    expect(commands.some((command) => command.includes('summary'))).toBe(false);
  });
});
