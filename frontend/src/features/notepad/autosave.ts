/**
 * The notepad's autosave: a debounce with a save queue.
 *
 * Every edit restarts a ~1 s timer; when typing pauses, the latest value is
 * saved. Saves never overlap (a slow disk cannot reorder them), a failed
 * save is retried with the newest value, and `flush` saves at once (on
 * unmount, before navigation, before the window closes).
 * Notes are therefore on disk about a second after the last keystroke,
 * which is what lets them survive a crash.
 */
import type { SaveStatus } from './types';

/** The pause after the last edit before saving, in ms. */
export const AUTOSAVE_DELAY_MS = 1000;

/** How long to wait before retrying a failed save, in ms. */
export const RETRY_DELAY_MS = 5000;

/** What an {@link Autosaver} needs. */
export interface AutosaverOptions<T> {
  /** Persists a value; rejects when it could not. */
  save: (value: T) => Promise<unknown>;
  /** Reports every status change, for the "Saved" indicator. */
  onStatus: (status: SaveStatus) => void;
  delay?: number;
}

/** Debounced, serialized saving of the latest value. */
export class Autosaver<T> {
  private pending: { value: T } | null = null;
  private timer: ReturnType<typeof setTimeout> | null = null;
  private queue: Promise<void> = Promise.resolve();

  constructor(private readonly options: AutosaverOptions<T>) {}

  /** Records an edit; the save happens once edits pause. */
  schedule(value: T): void {
    this.pending = { value };
    this.options.onStatus('pending');
    this.restartTimer(this.options.delay ?? AUTOSAVE_DELAY_MS);
  }

  /** Saves the latest unsaved value now; resolves when every save is done. */
  flush(): Promise<void> {
    this.stopTimer();
    const pending = this.pending;
    if (pending !== null) {
      this.pending = null;
      this.queue = this.queue.then(() => this.write(pending.value));
    }
    return this.queue;
  }

  /** True while an edit is waiting to be saved. */
  get dirty(): boolean {
    return this.pending !== null;
  }

  private async write(value: T): Promise<void> {
    this.options.onStatus('saving');
    try {
      await this.options.save(value);
      this.options.onStatus(this.pending === null ? 'saved' : 'pending');
    } catch {
      this.options.onStatus('error');
      // Retry with this value unless a newer edit has replaced it.
      this.pending ??= { value };
      this.restartTimer(RETRY_DELAY_MS);
    }
  }

  private restartTimer(delay: number): void {
    this.stopTimer();
    this.timer = setTimeout(() => void this.flush(), delay);
  }

  private stopTimer(): void {
    if (this.timer !== null) clearTimeout(this.timer);
    this.timer = null;
  }
}
