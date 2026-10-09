import type { Block } from '@blocknote/core';
import { describe, expect, test, vi } from 'vitest';

import { blocksToMarkdownSafely } from './blocknote-markdown';

/** A minimal block; the converter only counts blocks when it logs. */
const block = { id: 'block-1' } as Block;

describe('blocksToMarkdownSafely', () => {
  test('returns markdown when conversion succeeds', async () => {
    const editor = { blocksToMarkdownLossy: vi.fn(async () => '# Summary') };

    const result = await blocksToMarkdownSafely(editor, [], { source: 'test-success' });

    expect(result).toEqual({ markdown: '# Summary', ok: true });
    expect(editor.blocksToMarkdownLossy).toHaveBeenCalledTimes(1);
  });

  test('returns fallback markdown when conversion throws', async () => {
    const error = new Error('conversion failed');
    const editor = { blocksToMarkdownLossy: vi.fn(async () => Promise.reject(error)) };
    const consoleError = vi.spyOn(console, 'error').mockImplementation(() => {});

    const result = await blocksToMarkdownSafely(editor, [block], {
      source: 'test-fallback',
      fallbackMarkdown: 'existing markdown',
    });

    expect(result).toEqual({ markdown: 'existing markdown', ok: false });
    expect(consoleError).toHaveBeenCalledTimes(1);
    expect(consoleError).toHaveBeenCalledWith('Failed to convert BlockNote blocks to markdown', {
      source: 'test-fallback',
      blocksCount: 1,
      error,
    });
    consoleError.mockRestore();
  });

  test('omits markdown when conversion throws without fallback', async () => {
    const editor = {
      blocksToMarkdownLossy: vi.fn(async () => Promise.reject(new Error('conversion failed'))),
    };
    const consoleError = vi.spyOn(console, 'error').mockImplementation(() => {});

    const result = await blocksToMarkdownSafely(editor, [], { source: 'test-empty-fallback' });

    expect(result).toEqual({ markdown: undefined, ok: false });
    consoleError.mockRestore();
  });
});
