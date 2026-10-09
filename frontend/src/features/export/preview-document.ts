/**
 * The export preview of a document format (PDF, DOCX): Rust's HTML fragment
 * wrapped in a page styled from the current theme's tokens, for a
 * sandboxed `<iframe srcdoc>`. The fragment's inline light-theme colours on
 * secondary text (`data-role="meta"`) are overridden with the theme's.
 */

/** The tokens the preview page uses, read from the app's current theme. */
export interface PreviewPalette {
  fg: string;
  muted: string;
  border: string;
  surface: string;
  accent: string;
}

/** The colour tokens of `element`'s theme (`--fg`, `--fg-muted`, …). */
export function readPalette(element: Element): PreviewPalette {
  const style = getComputedStyle(element);
  const token = (name: string) => style.getPropertyValue(name).trim();
  return {
    fg: token('--fg'),
    muted: token('--fg-muted'),
    border: token('--border'),
    surface: token('--surface'),
    accent: token('--accent'),
  };
}

/** The page around `fragment`: the note column at the app's type scale. */
export function previewDocument(fragment: string, palette: PreviewPalette): string {
  const css = `
    :root { color-scheme: light dark; }
    html, body { margin: 0; background: ${palette.surface}; }
    body { padding: 28px 32px 40px; color: ${palette.fg};
      font: 14px/1.6 "Inter Variable", -apple-system, BlinkMacSystemFont, system-ui, sans-serif;
      -webkit-font-smoothing: antialiased; }
    body > div { color: ${palette.fg} !important; font-family: inherit !important; }
    h1 { font-size: 22px !important; letter-spacing: -0.02em; }
    h2 { font-size: 15px !important; margin-top: 24px !important; }
    h3, h4, h5, h6 { font-size: 13px; color: ${palette.muted}; margin: 16px 0 6px; }
    p, li { margin: 0 0 6px; }
    ul, ol { padding-left: 20px; margin: 0 0 10px; }
    a { color: ${palette.accent}; }
    blockquote { border-color: ${palette.border} !important; }
    [data-role="meta"] { color: ${palette.muted} !important; }
    ::selection { background: ${palette.accent}33; }
  `;
  return `<!doctype html><html><head><meta charset="utf-8"><style>${css}</style></head><body>${fragment}</body></html>`;
}
