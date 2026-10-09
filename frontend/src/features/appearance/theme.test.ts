import { beforeEach, describe, expect, test, vi } from 'vitest';

import { applyThemeToDocument, parseThemePreference, resolveTheme } from './theme';
import { THEME_STORAGE_KEY, webStorageThemeStore } from './theme-store';
import { createThemeStore } from './use-theme';

vi.mock('@tauri-apps/api/window', () => ({ getCurrentWindow: () => ({ setTheme: vi.fn() }) }));

/** A Web Storage area backed by a map. */
function memoryStorage(): Storage {
  const values = new Map<string, string>();
  return {
    get length() {
      return values.size;
    },
    clear: () => {
      values.clear();
    },
    getItem: (key) => values.get(key) ?? null,
    key: (index) => [...values.keys()][index] ?? null,
    removeItem: (key) => {
      values.delete(key);
    },
    setItem: (key, value) => {
      values.set(key, value);
    },
  };
}

describe('theme preference', () => {
  test('anything unreadable falls back to following the system', () => {
    expect(parseThemePreference('dark')).toBe('dark');
    expect(parseThemePreference('sepia')).toBe('system');
    expect(parseThemePreference(null)).toBe('system');
  });

  test('system resolves to whatever macOS is showing; explicit choices win', () => {
    expect(resolveTheme('system', true)).toBe('dark');
    expect(resolveTheme('system', false)).toBe('light');
    expect(resolveTheme('light', true)).toBe('light');
  });

  test('only an explicit choice pins data-theme, so system stays live', () => {
    const root = document.createElement('html');
    applyThemeToDocument('dark', root);
    expect(root.dataset.theme).toBe('dark');
    applyThemeToDocument('system', root);
    expect(root.dataset.theme).toBeUndefined();
  });
});

describe('theme state and first-paint cache', () => {
  let storage: Storage;
  beforeEach(() => {
    storage = memoryStorage();
    delete document.documentElement.dataset.theme;
  });

  test('the cached theme survives a restart under the key the boot script reads', () => {
    webStorageThemeStore(() => storage).save('light');
    expect(storage.getItem(THEME_STORAGE_KEY)).toBe('light');
    expect(webStorageThemeStore(() => storage).load()).toBe('light');
  });

  test('a storage failure never breaks the app', () => {
    const broken = webStorageThemeStore(() => {
      throw new Error('blocked');
    });
    expect(broken.load()).toBe('system');
    const consoleError = vi.spyOn(console, 'error').mockImplementation(() => {});
    expect(() => {
      broken.save('dark');
    }).not.toThrow();
    consoleError.mockRestore();
  });

  test('applying a theme switches the document at once and caches it for the next launch', () => {
    const useStore = createThemeStore(webStorageThemeStore(() => storage));
    useStore.getState().apply('dark');

    expect(useStore.getState().preference).toBe('dark');
    expect(document.documentElement.dataset.theme).toBe('dark');
    expect(storage.getItem(THEME_STORAGE_KEY)).toBe('dark');
  });
});
