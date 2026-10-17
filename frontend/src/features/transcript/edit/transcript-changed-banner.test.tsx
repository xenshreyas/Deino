import { render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import type { MeetingId } from '@/features/meetings';

import { transcriptCopy } from '../copy';
import { TranscriptChangedBanner } from './transcript-changed-banner';

const state = vi.hoisted(() => ({ stages: [] as { status: string }[] }));

vi.mock('@/features/notes', () => ({
  useEnhancedNotes: () => ({
    data: { isLatest: true, reasons: ['transcriptEdited'], version: { version: 1 } },
  }),
  useEnhanceAction: () => ({ run: vi.fn() }),
  useIsEnhancing: () => false,
}));
vi.mock('@/features/pipeline', () => ({
  usePipelineStatus: () => ({ data: state.stages }),
}));

const MEETING = 'meeting-1' as MeetingId;

describe('TranscriptChangedBanner', () => {
  beforeEach(() => {
    state.stages = [];
  });

  it('offers to regenerate the notes after the transcript changed', () => {
    state.stages = [{ status: 'succeeded' }];
    render(<TranscriptChangedBanner meetingId={MEETING} />);

    expect(screen.getByRole('button', { name: transcriptCopy.regenerate })).toBeTruthy();
  });

  it('stays away while processing rewrites the notes itself', () => {
    state.stages = [{ status: 'succeeded' }, { status: 'running' }, { status: 'pending' }];
    render(<TranscriptChangedBanner meetingId={MEETING} />);

    expect(screen.queryByRole('status')).toBeNull();
  });
});
