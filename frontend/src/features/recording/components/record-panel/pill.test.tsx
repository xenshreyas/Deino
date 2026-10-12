import { fireEvent, render, screen } from '@testing-library/react';
import type { ReactNode } from 'react';
import { describe, expect, it, vi } from 'vitest';

import type * as Generated from '@/lib/ipc/generated/recording';
import { TooltipProvider } from '@/ui/tooltip';

import { issueLabel, panelCopy } from '../../copy';
import { fill } from './level-meter';
import { Pill, type PillView } from './pill';
import type { PillActions } from './pill-controls';
import { SplitCard } from './split-card';

const LIVE: PillView = {
  phase: 'recording',
  elapsed: '12:04',
  levels: { microphone: 0.2, system: 0.1 },
  expanded: false,
  warning: null,
  ending: null,
};

function actions(): PillActions {
  return {
    onPause: vi.fn(),
    onResume: vi.fn(),
    onStop: vi.fn(),
    onMore: vi.fn(),
    onOpenNotes: vi.fn(),
    onKeep: vi.fn(),
  };
}

function withTooltips(node: ReactNode) {
  return render(<TooltipProvider>{node}</TooltipProvider>);
}

function renderPill(view: Partial<PillView>, handlers: PillActions = actions()) {
  withTooltips(<Pill view={{ ...LIVE, ...view }} actions={handlers} onPointerDown={vi.fn()} />);
  return handlers;
}

describe('Pill', () => {
  it('shows only the time while collapsed', () => {
    renderPill({});

    expect(screen.getByText('12:04')).toBeTruthy();
    expect(screen.queryByRole('button', { name: panelCopy.stop })).toBeNull();
  });

  it('reveals pause, stop, open notes and ⋯ on hover, and reports each one', () => {
    const handlers = renderPill({ expanded: true });

    fireEvent.click(screen.getByRole('button', { name: panelCopy.pause }));
    fireEvent.click(screen.getByRole('button', { name: panelCopy.stop }));
    fireEvent.click(screen.getByRole('button', { name: panelCopy.openNotes }));
    fireEvent.click(screen.getByRole('button', { name: panelCopy.more }));

    expect(handlers.onPause).toHaveBeenCalledOnce();
    expect(handlers.onStop).toHaveBeenCalledOnce();
    expect(handlers.onOpenNotes).toHaveBeenCalledOnce();
    expect(handlers.onMore).toHaveBeenCalledOnce();
  });

  it('keeps the device switches in ⋯, not on the pill (UX-29)', () => {
    renderPill({ expanded: true });

    expect(screen.getAllByRole('button')).toHaveLength(4);
    expect(screen.queryByRole('button', { name: panelCopy.microphone })).toBeNull();
    expect(screen.queryByRole('button', { name: panelCopy.screen })).toBeNull();
  });

  it('offers resume while paused', () => {
    const handlers = renderPill({ phase: 'paused', expanded: true });

    fireEvent.click(screen.getByRole('button', { name: panelCopy.resume }));

    expect(handlers.onResume).toHaveBeenCalledOnce();
  });

  it('names a health problem as an alert', () => {
    renderPill({ warning: 'No mic audio' });

    expect(screen.getByRole('alert').textContent).toBe('No mic audio');
  });

  it('replaces the controls with the countdown and keep recording', () => {
    const handlers = renderPill({ expanded: true, ending: { seconds: 11, reason: 'Call ended' } });

    expect(screen.getByText(panelCopy.ending(11))).toBeTruthy();
    expect(screen.queryByRole('button', { name: panelCopy.stop })).toBeNull();
    fireEvent.click(screen.getByRole('button', { name: panelCopy.keepRecording }));
    expect(handlers.onKeep).toHaveBeenCalledOnce();
  });

  it('says it is saving while the recording stops', () => {
    renderPill({ phase: 'stopping' });

    expect(screen.getByText(panelCopy.saving)).toBeTruthy();
    expect(screen.queryByText('12:04')).toBeNull();
  });
});

describe('SplitCard', () => {
  const offer: Generated.SplitOffer = {
    eventId: 'event-1' as Generated.CalendarEventId,
    title: 'Weekly sync',
    startsAt: '2026-10-11T10:00:00Z',
  };

  it('asks about the next meeting and reports both answers', () => {
    const onAccept = vi.fn();
    const onDismiss = vi.fn();
    withTooltips(
      <SplitCard offer={offer} busy={false} onAccept={onAccept} onDismiss={onDismiss} />,
    );

    expect(screen.getByText(panelCopy.splitTitle('Weekly sync'))).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: panelCopy.splitAccept }));
    fireEvent.click(screen.getByRole('button', { name: panelCopy.splitDismiss }));

    expect(onAccept).toHaveBeenCalledOnce();
    expect(onDismiss).toHaveBeenCalledOnce();
  });

  it('names an untitled event and disables the answers while one is pending', () => {
    withTooltips(
      <SplitCard offer={{ ...offer, title: ' ' }} busy onAccept={vi.fn()} onDismiss={vi.fn()} />,
    );

    expect(screen.getByText(panelCopy.splitTitle(panelCopy.untitledEvent))).toBeTruthy();
    const accept = screen.getByRole('button', { name: panelCopy.splitAccept });
    expect(accept.hasAttribute('disabled')).toBe(true);
  });
});

describe('the level meter', () => {
  it('spreads speech levels over the bar and clamps the ends', () => {
    expect(fill(0)).toBe(0);
    expect(fill(-1)).toBe(0);
    expect(fill(0.25)).toBeCloseTo(0.8);
    expect(fill(4)).toBe(1);
  });
});

describe('issueLabel', () => {
  it('says which side lost its audio', () => {
    expect(issueLabel({ kind: 'noAudio', channel: 'mic' })).toBe('No mic audio');
    expect(issueLabel({ kind: 'noAudio', channel: 'system' })).toBe('No call audio');
    expect(issueLabel({ kind: 'diskFull', channel: null })).toBe('Disk full');
  });
});
