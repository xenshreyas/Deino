import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

import { asMeetingId } from '@/features/meetings';

import { screenCopy } from '../copy';
import type { CaptureStatus, Snapshot } from '../types';
import { ImagesStripView, snapshotTime } from './images-strip';
import { capturedMeeting } from './screen-reminder';

const SLIDES: Snapshot[] = [
  { id: 1, at: 12_000, fileName: '0000012000.png', width: 960, height: 540, text: 'Q3' },
  { id: 2, at: 3_724_000, fileName: '0003724000.png', width: 960, height: 540, text: null },
];

function renderStrip(snapshots: Snapshot[] = SLIDES) {
  const onDelete = vi.fn();
  render(
    <ImagesStripView
      snapshots={snapshots}
      src={(snapshot) => `asset://${snapshot.fileName}`}
      deleting={false}
      onDelete={onDelete}
    />,
  );
  return onDelete;
}

describe('ImagesStripView', () => {
  it('shows nothing without snapshots', () => {
    renderStrip([]);
    expect(screen.queryByRole('region', { name: screenCopy.images.title })).toBeNull();
  });

  it('enlarges a snapshot and deletes it', () => {
    const onDelete = renderStrip();

    fireEvent.click(screen.getByRole('button', { name: screenCopy.images.open('1:02:04') }));
    fireEvent.click(screen.getByRole('button', { name: screenCopy.images.delete }));

    expect(onDelete).toHaveBeenCalledWith(SLIDES[1]);
  });
});

describe('snapshotTime', () => {
  it('reads like the transcript clock', () => {
    expect(snapshotTime(12_000)).toBe('00:12');
    expect(snapshotTime(3_724_000)).toBe('1:02:04');
  });
});

describe('capturedMeeting', () => {
  const meeting = asMeetingId('meeting-a');
  const capture = (state: CaptureStatus['state']): CaptureStatus => ({
    meeting,
    state,
    video: true,
    snapshots: 0,
  });

  it('names the meeting only while its screen is captured', () => {
    expect(capturedMeeting(capture('capturing'), meeting)).toBe(meeting);
    expect(capturedMeeting(capture('off'), meeting)).toBeNull();
    expect(capturedMeeting(undefined, meeting)).toBeNull();
    expect(capturedMeeting(capture('capturing'), null)).toBeNull();
  });
});
