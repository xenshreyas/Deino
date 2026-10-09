/**
 * Drag and drop of meetings and folders onto folders, over the native HTML
 * drag events (no library: two payload kinds and one kind of target).
 *
 * The payload travels in a custom MIME type per kind. While dragging over a
 * target the browser hides the data and shows only the types, so targets
 * decide from the type ({@link draggedKind}) and read the payload on drop.
 * Every drop also has a keyboard path: "Add to folder" and "Move to…" menus.
 */
import type { FolderId, MeetingId } from './types';

/** What is being dragged. */
export type DragPayload = { kind: 'meeting'; id: MeetingId } | { kind: 'folder'; id: FolderId };

/** The MIME type of each kind. */
const MIME = {
  meeting: 'application/x-deino-meeting',
  folder: 'application/x-deino-folder',
} as const satisfies Record<DragPayload['kind'], string>;

/** The part of `DataTransfer` this module uses, so tests can fake it. */
export type DragData = Pick<DataTransfer, 'setData' | 'getData' | 'types' | 'effectAllowed'>;

/** Starts a drag of `payload`: its MIME type, plus the title as plain text. */
export function writeDrag(data: DragData, payload: DragPayload, label: string): void {
  data.setData(MIME[payload.kind], payload.id);
  data.setData('text/plain', label);
  data.effectAllowed = payload.kind === 'meeting' ? 'copyMove' : 'move';
}

/** The kind being dragged, from the types alone (what `dragover` can see). */
export function draggedKind(types: readonly string[]): DragPayload['kind'] | null {
  if (types.includes(MIME.meeting)) return 'meeting';
  if (types.includes(MIME.folder)) return 'folder';
  return null;
}

/** Props that make an element drag `payload` (a meeting row, a tree row). */
export function dragSource(payload: DragPayload, label: string) {
  return {
    draggable: true,
    onDragStart: (event: { dataTransfer: DragData }) => {
      writeDrag(event.dataTransfer, payload, label);
    },
  };
}

/** Props that let a meeting row be dragged onto a folder. */
export function meetingDragSource(meeting: { id: MeetingId; title: string }) {
  return dragSource({ kind: 'meeting', id: meeting.id }, meeting.title);
}

/** The dropped payload, or `null` for a drop that is not ours. */
export function readDrag(data: Pick<DataTransfer, 'getData' | 'types'>): DragPayload | null {
  const kind = draggedKind([...data.types]);
  if (kind === null) return null;
  const id = data.getData(MIME[kind]);
  if (id === '') return null;
  // The IDs were written by `writeDrag` from typed IDs in this app.
  return kind === 'meeting' ? { kind, id: id as MeetingId } : { kind, id: id as FolderId };
}
