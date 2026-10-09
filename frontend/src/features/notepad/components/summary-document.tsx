/**
 * The AI summary as an editable document: its Markdown is turned into a
 * TipTap document by Rust (`deino-doc`), edited with the notepad's schema
 * and typography, and written back as Markdown when the user saves. It
 * replaced the BlockNote summary editor; NOT-02's enhanced notes replace it.
 */
import { EditorContent, useEditor } from '@tiptap/react';
import { forwardRef, useImperativeHandle, useState } from 'react';

import { Alert, AlertDescription } from '@/ui/alert';
import { SkeletonText } from '@/ui/skeleton';

import { notesApi } from '../api';
import { notepadCopy } from '../copy';
import { toNoteDoc } from '../doc-schema';
import { noteExtensions } from '../editor/extensions';
import { useMarkdownDoc } from '../queries';
import { summaryMarkdown } from '../summary-markdown';
import type { NoteDoc } from '../types';

/** What the meeting page's save and copy buttons call. */
export interface SummaryDocumentHandle {
  saveSummary: () => Promise<void>;
  getMarkdown: () => Promise<string>;
  isDirty: boolean;
}

/** Props of {@link SummaryDocument}. */
export interface SummaryDocumentProps {
  /** The stored summary, in any format it was saved in. */
  summary: unknown;
  onSave: (summary: { markdown: string }) => Promise<void>;
  onDirtyChange?: ((dirty: boolean) => void) | undefined;
}

/** The summary's editor: the notepad's schema, without mentions or a placeholder. */
function summaryEditorOptions(doc: NoteDoc, onUpdate: () => void) {
  return {
    extensions: noteExtensions({
      placeholder: '',
      resolveImageSrc: (src: string) => src,
      mentions: null,
      searchPeople: () => Promise.resolve([]),
      templates: { menu: null, deps: null },
    }),
    content: doc,
    editorProps: {
      attributes: {
        class: 'note-content',
        role: 'textbox',
        'aria-multiline': 'true',
        'aria-label': notepadCopy.summary,
      },
    },
    onUpdate,
  };
}

type EditorProps = Omit<SummaryDocumentProps, 'summary'> & { doc: NoteDoc; markdown: string };

const SummaryEditor = forwardRef<SummaryDocumentHandle, EditorProps>(function SummaryEditor(
  { doc, markdown, onSave, onDirtyChange },
  ref,
) {
  const [dirty, setDirty] = useState(false);
  const markDirty = (next: boolean) => {
    setDirty(next);
    onDirtyChange?.(next);
  };
  const editor = useEditor(
    summaryEditorOptions(doc, () => {
      markDirty(true);
    }),
  );
  const getMarkdown = () =>
    dirty ? notesApi.docToMarkdown(toNoteDoc(editor.getJSON())) : Promise.resolve(markdown);
  useImperativeHandle(ref, () => ({
    isDirty: dirty,
    getMarkdown,
    saveSummary: async () => {
      if (!dirty) return;
      await onSave({ markdown: await getMarkdown() });
      markDirty(false);
    },
  }));
  return <EditorContent editor={editor} />;
});

/** The summary, loading while Rust converts it. */
export const SummaryDocument = forwardRef<SummaryDocumentHandle, SummaryDocumentProps>(
  function SummaryDocument({ summary, onSave, onDirtyChange }, ref) {
    const markdown = summaryMarkdown(summary);
    const doc = useMarkdownDoc(markdown);
    if (doc.isPending) return <SkeletonText lines={5} />;
    if (doc.isError) {
      return (
        <Alert variant="destructive">
          <AlertDescription>{doc.error.message}</AlertDescription>
        </Alert>
      );
    }
    return (
      <SummaryEditor
        key={markdown}
        ref={ref}
        doc={doc.data}
        markdown={markdown}
        onSave={onSave}
        onDirtyChange={onDirtyChange}
      />
    );
  },
);
