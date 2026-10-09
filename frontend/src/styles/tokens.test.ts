/**
 * Guards the design tokens: both themes define the same tokens, the two
 * copies of the dark theme never drift apart, and every text pair the UI
 * uses meets WCAG AA (docs/references/ui.md §2.3).
 */
import { describe, expect, test } from 'vitest';

// `vite.config.ts` lets this one stylesheet through Vitest's CSS stub.
import tokensCss from './tokens.css?raw';

/** The `--name: value;` declarations inside the first block that `selector` opens. */
function block(selector: string): Map<string, string> {
  const start = tokensCss.indexOf(selector);
  if (start === -1) throw new Error(`tokens.css has no ${selector} block`);
  const open = tokensCss.indexOf('{', start);
  const close = tokensCss.indexOf('}', open);
  const declarations = tokensCss.slice(open + 1, close).matchAll(/--([\w-]+):\s*([^;]+);/g);
  return new Map([...declarations].map(([, name = '', value = '']) => [name, value.trim()]));
}

const light = block(":root,\n[data-theme='light']");
const systemDark = block(":root:not([data-theme='light'])");
const explicitDark = block("[data-theme='dark'] {");

/** Relative luminance of a `#rrggbb` colour (WCAG 2.x). */
function luminance(hex: string): number {
  const channels = [1, 3, 5].map(
    (offset) => Number.parseInt(hex.slice(offset, offset + 2), 16) / 255,
  );
  const [r = 0, g = 0, b = 0] = channels.map((c) =>
    c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4,
  );
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

/** WCAG contrast ratio between two tokens of one theme. */
function contrast(theme: Map<string, string>, foreground: string, background: string): number {
  const fg = theme.get(foreground);
  const bg = theme.get(background);
  if (fg === undefined || bg === undefined)
    throw new Error(`missing ${foreground} or ${background}`);
  const [high, low] = [luminance(fg), luminance(bg)].sort((a, b) => b - a);
  return ((high ?? 0) + 0.05) / ((low ?? 0) + 0.05);
}

/** Text pairs that must reach 4.5:1 (body text) in both themes. */
const TEXT_PAIRS: readonly [string, string][] = [
  ['fg', 'bg'],
  ['fg', 'bg-subtle'],
  ['fg', 'bg-muted'],
  ['fg', 'surface'],
  ['fg', 'surface-raised'],
  ['fg-muted', 'bg'],
  ['fg-muted', 'bg-subtle'],
  ['fg-muted', 'bg-muted'],
  ['fg-muted', 'surface'],
  ['ai-text', 'bg'],
  ['user-text', 'bg'],
  ['accent-fg', 'accent'],
  ['accent', 'bg'],
  ['fg', 'accent-soft'],
  ['danger', 'bg'],
  ['danger', 'danger-soft'],
  ['warning-text', 'warning-soft'],
  ['success', 'bg'],
  ['fg', 'selection'],
  ['bg', 'fg'],
];

/** Non-text pairs (placeholders, the record dot, focus ring) need 3:1. */
const NON_TEXT_PAIRS: readonly [string, string][] = [
  ['fg-subtle', 'bg'],
  ['recording', 'bg'],
  ['focus-ring', 'bg'],
];

describe('design tokens', () => {
  test('the explicit dark theme is an exact copy of the system dark theme', () => {
    expect(Object.fromEntries(explicitDark)).toEqual(Object.fromEntries(systemDark));
  });

  test('every colour token has a dark value', () => {
    const colours = [...light.keys()].filter((name) => light.get(name)?.startsWith('#') === true);
    expect(colours.filter((name) => !systemDark.has(name))).toEqual([]);
  });

  test.each([
    ['light', light],
    ['dark', systemDark],
  ] as const)('every text pair meets WCAG AA in the %s theme', (_name, theme) => {
    const failing = TEXT_PAIRS.filter(([fg, bg]) => contrast(theme, fg, bg) < 4.5);
    expect(failing).toEqual([]);
  });

  test.each([
    ['light', light],
    ['dark', systemDark],
  ] as const)('non-text marks reach 3:1 in the %s theme', (_name, theme) => {
    const failing = NON_TEXT_PAIRS.filter(([fg, bg]) => contrast(theme, fg, bg) < 3);
    expect(failing).toEqual([]);
  });

  test('the speaker colours are readable as names on the canvas', () => {
    for (const theme of [light, systemDark]) {
      for (const speaker of [1, 2, 3, 4, 5, 6]) {
        expect(contrast(theme, `speaker-${speaker}`, 'bg')).toBeGreaterThanOrEqual(4.5);
      }
    }
  });
});
