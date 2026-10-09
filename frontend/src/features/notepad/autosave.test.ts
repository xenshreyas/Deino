import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { AUTOSAVE_DELAY_MS, Autosaver, RETRY_DELAY_MS } from './autosave';
import type { SaveStatus } from './types';

function harness(save: (value: string) => Promise<unknown>) {
  const statuses: SaveStatus[] = [];
  const saver = new Autosaver({ save, onStatus: (status) => statuses.push(status) });
  return { saver, statuses };
}

describe('Autosaver', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });
  afterEach(() => {
    vi.useRealTimers();
  });

  it('saves the latest value once typing pauses for a second', async () => {
    const save = vi.fn(() => Promise.resolve());
    const { saver, statuses } = harness(save);

    saver.schedule('h');
    await vi.advanceTimersByTimeAsync(AUTOSAVE_DELAY_MS - 1);
    saver.schedule('hi');
    await vi.advanceTimersByTimeAsync(AUTOSAVE_DELAY_MS - 1);
    expect(save).not.toHaveBeenCalled();

    await vi.advanceTimersByTimeAsync(1);
    expect(save).toHaveBeenCalledExactlyOnceWith('hi');
    expect(statuses.at(-1)).toBe('saved');
  });

  it('flush saves at once and leaves nothing behind', async () => {
    const save = vi.fn(() => Promise.resolve());
    const { saver } = harness(save);

    saver.schedule('notes');
    await saver.flush();
    await vi.advanceTimersByTimeAsync(AUTOSAVE_DELAY_MS * 2);

    expect(save).toHaveBeenCalledExactlyOnceWith('notes');
    expect(saver.dirty).toBe(false);
  });

  it('never overlaps saves, so they land in order', async () => {
    const landed: string[] = [];
    let release: () => void = () => undefined;
    const save = vi.fn(async (value: string) => {
      if (value === 'first') await new Promise<void>((resolve) => (release = resolve));
      landed.push(value);
    });
    const { saver } = harness(save);

    saver.schedule('first');
    const first = saver.flush();
    saver.schedule('second');
    const second = saver.flush();
    await vi.advanceTimersByTimeAsync(0);
    expect(landed).toEqual([]);
    release();
    await Promise.all([first, second]);

    expect(landed).toEqual(['first', 'second']);
  });

  it('retries a failed save with the newest value', async () => {
    const save = vi
      .fn<(value: string) => Promise<unknown>>()
      .mockRejectedValueOnce(new Error('disk full'))
      .mockResolvedValue(undefined);
    const { saver, statuses } = harness(save);

    saver.schedule('draft');
    await vi.advanceTimersByTimeAsync(AUTOSAVE_DELAY_MS);
    expect(statuses.at(-1)).toBe('error');

    await vi.advanceTimersByTimeAsync(RETRY_DELAY_MS);
    expect(save).toHaveBeenLastCalledWith('draft');
    expect(statuses.at(-1)).toBe('saved');
  });
});
