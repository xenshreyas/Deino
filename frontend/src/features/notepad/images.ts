/**
 * Getting images into a note: pasted or dropped files are sent to Rust as
 * bytes (base64), and files dropped through Tauri's native drag and drop
 * by the path the drop reported, which Rust accepts only for that drop.
 * Either way Rust checks the content, names the file and stores it in the
 * meeting's folder; the note gets the returned `images/<id>.<ext>` source.
 */
import type { EditorView } from '@tiptap/pm/view';
import { toast } from 'sonner';

import { IpcError } from '@/lib/ipc/result';

import { notesApi } from './api';
import { notepadCopy } from './copy';
import type { NoteTarget } from './target';

/** File types the notepad accepts, by MIME type and by extension. */
const IMAGE_TYPES = /^image\/(png|jpeg|gif|webp)$/;
const IMAGE_EXTENSIONS = /\.(png|jpe?g|gif|webp)$/i;

/** True for a dropped path that looks like an image the notepad accepts. */
export function isImagePath(path: string): boolean {
  return IMAGE_EXTENSIONS.test(path);
}

/** The image files among `files`. */
export function imageFiles(files: FileList | null | undefined): File[] {
  return [...(files ?? [])].filter((file) => IMAGE_TYPES.test(file.type));
}

/** A file's bytes as base64, without a `data:` prefix. */
export async function toBase64(file: Blob): Promise<string> {
  const bytes = new Uint8Array(await file.arrayBuffer());
  let binary = '';
  const chunk = 0x8000;
  for (let start = 0; start < bytes.length; start += chunk) {
    binary += String.fromCharCode(...bytes.subarray(start, start + chunk));
  }
  return btoa(binary);
}

/** Inserts an image node at `pos`, or over the selection. */
function insertImage(view: EditorView, src: string, pos: number | null): void {
  const node = view.state.schema.nodes.image?.create({ src });
  if (node === undefined) return;
  const { tr } = view.state;
  view.dispatch(pos === null ? tr.replaceSelectionWith(node) : tr.insert(pos, node));
}

/** Stores each image with `store` and inserts it; failures become a toast. */
async function insertStored(
  view: EditorView,
  sources: readonly (() => Promise<string>)[],
  pos: number | null,
): Promise<void> {
  for (const store of sources) {
    try {
      insertImage(view, await store(), pos);
    } catch (error) {
      toast.error(notepadCopy.imageFailedTitle, { description: IpcError.from(error).message });
    }
  }
}

/** Uploads pasted or dropped files into the target's meeting. */
export function insertFiles(
  view: EditorView,
  target: NoteTarget,
  files: readonly File[],
  pos: number | null,
): Promise<void> {
  const sources = files.map((file) => async () => {
    const { meetingId } = await target.resolve();
    return (await notesApi.saveImage(meetingId, await toBase64(file))).src;
  });
  return insertStored(view, sources, pos);
}

/** Attaches files dropped through Tauri's native drag and drop. */
export function insertDroppedPaths(
  view: EditorView,
  target: NoteTarget,
  paths: readonly string[],
  pos: number | null,
): Promise<void> {
  const sources = paths.filter(isImagePath).map((path) => async () => {
    const { meetingId } = await target.resolve();
    return (await notesApi.attachDroppedImage(meetingId, path)).src;
  });
  return insertStored(view, sources, pos);
}
