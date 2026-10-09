/**
 * Decides what a file drop onto the window means for audio import.
 *
 * Pure so the rules are testable without Tauri: the drop target component
 * turns the outcome into a dialog or a toast.
 */
import { isAudioExtension } from '@/constants/audioFormats';

/** What the shell should do with a set of dropped file paths. */
export type AudioDropOutcome =
  | { kind: 'import-disabled' }
  | { kind: 'audio'; path: string }
  | { kind: 'unsupported' }
  | { kind: 'nothing' };

/** Lower-case extension of `path` without the dot, or `undefined`. */
function extensionOf(path: string): string | undefined {
  const dot = path.lastIndexOf('.');
  return dot === -1 ? undefined : path.slice(dot + 1).toLowerCase();
}

/**
 * Classifies a drop. The first audio file wins; other files are ignored.
 *
 * @param paths - absolute paths from Tauri's `tauri://drag-drop` event
 * @param importEnabled - whether the "Import Audio & Retranscribe" beta is on
 */
export function classifyAudioDrop(
  paths: readonly string[],
  importEnabled: boolean,
): AudioDropOutcome {
  if (!importEnabled) return { kind: 'import-disabled' };
  const audio = paths.find((path) => {
    const extension = extensionOf(path);
    return extension !== undefined && isAudioExtension(extension);
  });
  if (audio !== undefined) return { kind: 'audio', path: audio };
  return paths.length > 0 ? { kind: 'unsupported' } : { kind: 'nothing' };
}
