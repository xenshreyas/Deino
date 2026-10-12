/**
 * The files imports accept, by extension. Mirrors `AUDIO_EXTENSIONS` and
 * `VIDEO_EXTENSIONS` in `crates/deino-core/src/import/model.rs`; Rust has
 * the last word (`check` refuses anything else), this list only decides
 * what a window drop offers to import.
 */

/** Audio-only formats. */
export const AUDIO_EXTENSIONS = [
  'm4a',
  'mp3',
  'wav',
  'aac',
  'aif',
  'aiff',
  'caf',
  'flac',
  'ogg',
  'oga',
  'opus',
] as const;

/** Video formats; the video is kept for playback. */
export const VIDEO_EXTENSIONS = ['mp4', 'mov', 'm4v', 'mkv', 'webm'] as const;

const ACCEPTED: ReadonlySet<string> = new Set([...AUDIO_EXTENSIONS, ...VIDEO_EXTENSIONS]);

/** Lower-case extension of `path` without the dot, or `undefined`. */
function extensionOf(path: string): string | undefined {
  const name = path.slice(path.lastIndexOf('/') + 1);
  const dot = name.lastIndexOf('.');
  return dot <= 0 ? undefined : name.slice(dot + 1).toLowerCase();
}

/** Whether imports accept the file at `path`. */
export function isImportable(path: string): boolean {
  const extension = extensionOf(path);
  return extension !== undefined && ACCEPTED.has(extension);
}

/** The accepted formats for a sentence: "MP3, WAV, … and WebM". */
export function formatList(): string {
  const names = [...AUDIO_EXTENSIONS, ...VIDEO_EXTENSIONS].map((extension) =>
    extension === 'webm' ? 'WebM' : extension.toUpperCase(),
  );
  return `${names.slice(0, -1).join(', ')} and ${names.at(-1) ?? ''}`;
}
