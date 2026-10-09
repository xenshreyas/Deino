/**
 * Image files dropped on the window through Tauri's native drag and drop,
 * which reports paths (not files) and never reaches the DOM's drop event.
 * A drop over the editor inserts its images where it landed.
 */
import { listen } from '@tauri-apps/api/event';
import type { Editor } from '@tiptap/react';

import { useMountEffect } from '@/hooks/useMountEffect';

import { insertDroppedPaths, isImagePath } from '../images';
import type { NoteTarget } from '../target';

/** Payload of Tauri's `tauri://drag-drop` event (physical pixels). */
interface DropPayload {
  paths: string[];
  position: { x: number; y: number };
}

/** The document position under a drop, or `undefined` when it missed the editor. */
function dropPosition(editor: Editor, payload: DropPayload): number | null | undefined {
  const left = payload.position.x / window.devicePixelRatio;
  const top = payload.position.y / window.devicePixelRatio;
  const box = editor.view.dom.getBoundingClientRect();
  const inside = left >= box.left && left <= box.right && top >= box.top && top <= box.bottom;
  if (!inside) return undefined;
  return editor.view.posAtCoords({ left, top })?.pos ?? null;
}

/** Inserts images dropped natively over `editor`. */
export function useNativeImageDrop(editor: Editor, target: NoteTarget): void {
  useMountEffect(() => {
    const pending = listen<DropPayload>('tauri://drag-drop', ({ payload }) => {
      if (!payload.paths.some(isImagePath) || editor.isDestroyed) return;
      const pos = dropPosition(editor, payload);
      if (pos !== undefined) void insertDroppedPaths(editor.view, target, payload.paths, pos);
    });
    return () => {
      void pending.then((unlisten) => {
        unlisten();
      });
    };
  });
}
