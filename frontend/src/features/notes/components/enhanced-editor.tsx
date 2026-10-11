/**
 * The enhanced notes as an editable document (NOT-02b): the notepad's
 * schema (so `origin` marks and `citation` nodes render), plus the
 * extension that marks edited AI text. The newest version autosaves about
 * a second after typing pauses; older versions are read-only. Citations are
 * hidden in the text and surface through the magnifier (`CitedFrame`).
 */
import type { Extensions } from '@tiptap/core';
import { type Editor, EditorContent, useEditor } from '@tiptap/react';
import { useState } from 'react';

import type { MeetingId } from '@/features/meetings';
import {
  Autosaver,
  type NoteExtensionOptions,
  type SaveStatus,
  toNoteDoc,
} from '@/features/notepad';
import { useMountEffect } from '@/hooks/useMountEffect';

import { enhanceApi } from '../api';
import { CitedFrame } from '../citations/cited-frame';
import { notesCopy } from '../copy';
import { EditedAiText } from '../editor/edited-ai-text';
import type { EnhancedNotesView } from '../types';

/** The notepad's extension factory, as `loadNoteExtensions` resolves it. */
export type NoteExtensionFactory = (options: NoteExtensionOptions) => Extensions;

/** Props of {@link EnhancedEditor}. */
export interface EnhancedEditorProps {
  meetingId: MeetingId;
  view: EnhancedNotesView;
  noteExtensions: NoteExtensionFactory;
  onStatus: (status: SaveStatus) => void;
}

/** The note schema without the notepad's menus: notes are read and edited, not composed. */
function extensions(noteExtensions: NoteExtensionFactory): Extensions {
  const schema = noteExtensions({
    placeholder: '',
    resolveImageSrc: (src) => src,
    mentions: null,
    searchPeople: () => Promise.resolve([]),
    templates: { menu: null, deps: null },
  });
  return [...schema, EditedAiText];
}

/** The document, its autosave and its magnifier. */
export function EnhancedEditor({ meetingId, view, noteExtensions, onStatus }: EnhancedEditorProps) {
  const version = view.version.version;
  const [saver] = useState(
    () =>
      new Autosaver<Editor>({
        save: (editor) => enhanceApi.save(meetingId, version, toNoteDoc(editor.getJSON())),
        onStatus,
      }),
  );
  const editor = useEditor({
    extensions: extensions(noteExtensions),
    content: view.doc,
    editable: view.isLatest,
    immediatelyRender: true,
    shouldRerenderOnTransaction: false,
    editorProps: { attributes: editorAttributes(view.isLatest) },
    onUpdate: ({ editor: updated }) => {
      saver.schedule(updated);
    },
  });
  useMountEffect(() => () => {
    void saver.flush();
  });
  return (
    <CitedFrame meetingId={meetingId} editor={editor}>
      <EditorContent editor={editor} />
    </CitedFrame>
  );
}

function editorAttributes(editable: boolean): Record<string, string> {
  return {
    class: 'note-content enhanced-notes',
    role: 'textbox',
    'aria-multiline': 'true',
    'aria-readonly': String(!editable),
    'aria-label': notesCopy.editorLabel,
  };
}
