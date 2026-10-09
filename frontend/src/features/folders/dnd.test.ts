import { describe, expect, it } from 'vitest';

import { type DragData, draggedKind, meetingDragSource, readDrag, writeDrag } from './dnd';
import type { FolderId, MeetingId } from './types';

/** A `DataTransfer` stand-in: what the browser keeps between dragstart and drop. */
function transfer(): DragData {
  const data = new Map<string, string>();
  return {
    setData: (format, value) => {
      data.set(format, value);
    },
    getData: (format) => data.get(format) ?? '',
    get types() {
      return [...data.keys()];
    },
    effectAllowed: 'none',
  };
}

describe('drag payloads', () => {
  it('round-trip a meeting and a folder through their own MIME types', () => {
    const meeting = transfer();
    const folder = transfer();

    writeDrag(meeting, { kind: 'meeting', id: 'meeting-1' as MeetingId }, 'Standup');
    writeDrag(folder, { kind: 'folder', id: 'folder-1' as FolderId }, 'Work');

    expect(readDrag(meeting)).toEqual({ kind: 'meeting', id: 'meeting-1' });
    expect(readDrag(folder)).toEqual({ kind: 'folder', id: 'folder-1' });
    expect(meeting.getData('text/plain')).toBe('Standup');
    expect([meeting.effectAllowed, folder.effectAllowed]).toEqual(['copyMove', 'move']);
  });

  it('tell the kind from the types alone, as dragover must', () => {
    const data = transfer();
    meetingDragSource({ id: 'meeting-1' as MeetingId, title: 'Standup' }).onDragStart({
      dataTransfer: data,
    });

    expect(draggedKind(data.types)).toBe('meeting');
    expect(draggedKind(['text/plain', 'Files'])).toBeNull();
  });

  it('ignore drops that are not Deino meetings or folders', () => {
    const text = transfer();
    text.setData('text/plain', 'hello');

    expect(readDrag(text)).toBeNull();
  });
});
