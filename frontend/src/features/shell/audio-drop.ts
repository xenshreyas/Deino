/**
 * Decides what a file drop onto the window means for importing a recording.
 *
 * Pure so the rules are testable without Tauri: the drop target component
 * turns the outcome into the import dialog or a toast.
 */
import { isImportable } from '@/features/import';

/** What the shell should do with a set of dropped file paths. */
export type AudioDropOutcome =
  { kind: 'audio'; path: string } | { kind: 'unsupported' } | { kind: 'nothing' };

/** Image files the notepad accepts (`notepad/images.ts`). */
const NOTE_IMAGE = /\.(png|jpe?g|gif|webp)$/i;

/**
 * Classifies a drop. The first audio or video file wins; other files are
 * ignored.
 *
 * @param paths - absolute paths from Tauri's `tauri://drag-drop` event
 */
export function classifyAudioDrop(paths: readonly string[]): AudioDropOutcome {
  // Images belong to the notepad, which inserts them where they land (NOT-01).
  if (paths.length > 0 && paths.every((path) => NOTE_IMAGE.test(path))) return { kind: 'nothing' };
  const recording = paths.find(isImportable);
  if (recording !== undefined) return { kind: 'audio', path: recording };
  return paths.length > 0 ? { kind: 'unsupported' } : { kind: 'nothing' };
}
