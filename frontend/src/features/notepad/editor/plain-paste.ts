/**
 * ⌘⇧V pastes as plain text: no fonts, colours or links from the source,
 * just the words, split into paragraphs at line breaks. ProseMirror already
 * treats a shift-paste event as plain text; WKWebView does not send one for
 * ⌘⇧V, so the shortcut reads the clipboard itself.
 */
import { type Editor, Extension } from '@tiptap/core';

/** Pastes `text` at the selection exactly as typed text would land. */
export function pastePlainText(editor: Editor, text: string): boolean {
  if (text === '') return false;
  return editor.view.pasteText(text);
}

/** Reads the clipboard's text; empty when the webview refuses. */
async function clipboardText(): Promise<string> {
  try {
    return await navigator.clipboard.readText();
  } catch {
    return '';
  }
}

/** The ⌘⇧V shortcut. */
export const PlainTextPaste = Extension.create({
  name: 'plainTextPaste',

  addKeyboardShortcuts() {
    return {
      'Mod-Shift-v': ({ editor }) => {
        void clipboardText().then((text) => pastePlainText(editor, text));
        return true;
      },
    };
  },
});
