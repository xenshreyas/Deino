import { render, screen } from '@testing-library/react';
import { afterEach, describe, expect, test, vi } from 'vitest';

import { asMeetingId } from '@/features/meetings';
import { useShellLayout } from '@/features/shell';

import { ContextRail } from './context-rail';

// The real sections reach the IPC; this test is about when the rail shows.
vi.mock('../slots', () => ({
  MEETING_RAIL: [
    { id: 'people', Component: () => <p>People section</p> },
    { id: 'follow-up', Component: () => <p>Follow-up section</p> },
  ],
}));

const MEETING = asMeetingId('meeting-fixture-standup');

describe('the context rail', () => {
  afterEach(() => {
    useShellLayout.setState({ rightPanelOpen: false });
  });

  test('renders its sections in order, shown only from a 1100px meeting area', () => {
    render(<ContextRail meetingId={MEETING} />);

    // `hidden` keeps it out of narrow pages; the container query shows it when wide.
    const rail = screen.getByLabelText('Meeting context');
    expect(rail.className).toContain('hidden');
    expect(rail.className).toContain('@min-[1100px]/meeting:flex');
    expect(rail.textContent).toBe('People sectionFollow-up section');
  });

  test('gives way to the right panel, which shows the same people and follow-up', () => {
    useShellLayout.setState({ rightPanelOpen: true });
    render(<ContextRail meetingId={MEETING} />);

    expect(screen.queryByLabelText('Meeting context')).toBeNull();
    expect(screen.queryByText('People section')).toBeNull();
  });
});
