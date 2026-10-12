import { QueryClientProvider } from '@tanstack/react-query';
import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  RouterProvider,
} from '@tanstack/react-router';
import { fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import type { CalendarEventId } from '@/lib/ipc/generated/calendar';
import type { MeetingId } from '@/lib/ipc/generated/meetings';
import { createQueryClient } from '@/lib/ipc';
import { TooltipProvider } from '@/ui/tooltip';

import type { Brief } from '../types';
import { BriefCard } from './brief-card';
import { EventBrief } from './brief-slot';

const commands = vi.hoisted(() => ({
  forEvent: vi.fn(),
  forMeeting: vi.fn(),
  markRead: vi.fn(),
  giveFeedback: vi.fn(),
}));
vi.mock('@/lib/ipc/generated/briefs', () => ({ commands }));

const EVENT = 'event-1' as CalendarEventId;

function brief(overrides: Partial<Brief> = {}): Brief {
  return {
    eventId: EVENT,
    bullets: [
      {
        text: 'The licence ends in January.',
        citations: [
          {
            kind: 'meeting',
            meetingId: 'meeting-1' as MeetingId,
            title: 'Discovery call',
            at: '2026-09-30T15:00:00Z',
          },
        ],
      },
      { text: 'The agenda covers pricing.', citations: [{ kind: 'event' }] },
    ],
    generatedAt: '2026-10-12T13:00:00Z',
    readAt: null,
    feedback: null,
    ...overrides,
  };
}

/** Renders `card` with the tooltip context its icon button needs. */
function renderCard(card: React.ReactElement) {
  render(<TooltipProvider>{card}</TooltipProvider>);
}

describe('BriefCard', () => {
  it('shows the points with their citations and collapses to one line', () => {
    renderCard(
      <BriefCard brief={brief()} defaultExpanded onNotUseful={vi.fn()} onOpenCitation={vi.fn()} />,
    );

    expect(screen.getByText('The licence ends in January.')).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Discovery call' })).toBeTruthy();
    expect(screen.getByText('Invitation')).toBeTruthy();

    fireEvent.click(screen.getByRole('button', { name: 'Hide brief' }));
    expect(screen.queryByText('The licence ends in January.')).toBeNull();
    expect(screen.getByText('· 2 points')).toBeTruthy();
  });

  it('opens a citation and sends the thumbs down', () => {
    const onOpen = vi.fn();
    const onNotUseful = vi.fn();
    renderCard(
      <BriefCard
        brief={brief()}
        defaultExpanded
        onNotUseful={onNotUseful}
        onOpenCitation={onOpen}
      />,
    );

    fireEvent.click(screen.getByRole('button', { name: 'Discovery call' }));
    fireEvent.click(screen.getByRole('button', { name: 'Not useful' }));

    expect(onOpen).toHaveBeenCalledWith(expect.objectContaining({ kind: 'meeting' }));
    expect(onNotUseful).toHaveBeenCalledOnce();
  });

  it('starts collapsed once read and disables the thumbs down once given', () => {
    renderCard(
      <BriefCard
        brief={brief({ readAt: '2026-10-12T13:05:00Z', feedback: 'not_useful' })}
        defaultExpanded={false}
        onNotUseful={vi.fn()}
        onOpenCitation={vi.fn()}
      />,
    );

    expect(screen.queryByText('The licence ends in January.')).toBeNull();
    expect(screen.getByRole<HTMLButtonElement>('button', { name: 'Thanks, noted' }).disabled).toBe(
      true,
    );
  });
});

/** `EventBrief` at `/`, inside a router and a query client. */
function renderEventBrief() {
  const root = createRootRoute();
  const home = createRoute({
    getParentRoute: () => root,
    path: '/',
    component: () => <EventBrief eventId={EVENT} />,
  });
  const router = createRouter({
    routeTree: root.addChildren([home]),
    history: createMemoryHistory({ initialEntries: ['/'] }),
  });
  render(
    <QueryClientProvider client={createQueryClient()}>
      <TooltipProvider>
        <RouterProvider router={router} />
      </TooltipProvider>
    </QueryClientProvider>,
  );
}

describe('EventBrief', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    commands.markRead.mockResolvedValue({ status: 'ok', data: null });
  });

  it('shows an unread brief open and marks it read', async () => {
    commands.forEvent.mockResolvedValue({ status: 'ok', data: brief() });

    renderEventBrief();

    expect(await screen.findByText('The licence ends in January.')).toBeTruthy();
    expect(commands.markRead).toHaveBeenCalledWith(EVENT);
  });

  it('renders nothing without a brief', async () => {
    commands.forEvent.mockResolvedValue({ status: 'ok', data: null });

    renderEventBrief();

    await vi.waitFor(() => {
      expect(commands.forEvent).toHaveBeenCalled();
    });
    expect(screen.queryByRole('region', { name: 'Brief' })).toBeNull();
    expect(commands.markRead).not.toHaveBeenCalled();
  });
});
